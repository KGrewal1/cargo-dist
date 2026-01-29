//! Code for generating Python wheels from binaries

use axoasset::LocalAsset;
use camino::Utf8PathBuf;
use cargo_dist_schema::TripleName;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use tracing::{info, warn};

use super::InstallerInfo;
use crate::{DistGraph, DistResult};

/// Info about a Python wheel installer
#[derive(Debug, Clone, Serialize)]
pub struct PypiWheelInstallerInfo {
    /// Python package name (normalized: dashes to underscores)
    pub package_name: String,
    /// Package version
    pub package_version: String,
    /// Package description
    pub package_desc: Option<String>,
    /// Package license
    pub package_license: Option<String>,
    /// Package authors
    pub package_authors: Vec<String>,
    /// Homepage URL
    pub homepage_url: Option<String>,
    /// Target platform for this wheel
    pub target_triple: TripleName,
    /// Wheel output path
    pub dest_path: Utf8PathBuf,
    /// Generic installer info
    pub inner: InstallerInfo,
}

struct RecordEntry {
    path: String,
    hash: String,
    size: usize,
}

pub(crate) fn write_wheel(dist: &DistGraph, info: &PypiWheelInstallerInfo) -> DistResult<()> {
    let wheel_filename = wheel_filename(info);
    let wheel_path = info.dest_path.join(&wheel_filename);

    // Create temporary directory for wheel contents
    let temp_dir = info.dest_path.join(format!("{}.temp", wheel_filename));
    std::fs::create_dir_all(&temp_dir)?;

    // Normalize package name (replace dashes with underscores)
    let normalized_name = info.package_name.replace('-', "_");

    // Track all files for RECORD generation
    let mut record_entries = Vec::new();

    // 1. Create .data/scripts/ directory for binaries
    // This is the standard location for executables in wheels
    // pip will automatically install them to the scripts directory (in PATH)
    let scripts_dir = temp_dir.join(format!("{}-{}.data", normalized_name, info.package_version)).join("scripts");
    std::fs::create_dir_all(&scripts_dir)?;

    for artifact in &info.inner.artifacts {
        for exe in &artifact.executables {
            // Find the binary in the dist directory
            let exe_path = dist
                .dist_dir
                .join(artifact.target_triple.as_str())
                .join(exe.clone());

            // Read binary content
            let mut exe_file = File::open(&exe_path)?;
            let mut exe_content = Vec::new();
            exe_file.read_to_end(&mut exe_content)?;

            // Write binary to scripts directory
            let dest_exe_path = scripts_dir.join(exe);
            std::fs::write(&dest_exe_path, &exe_content)?;

            // Set executable permissions on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let metadata = std::fs::metadata(&dest_exe_path)?;
                let mut permissions = metadata.permissions();
                permissions.set_mode(0o755);
                std::fs::set_permissions(&dest_exe_path, permissions)?;
            }

            add_file_to_record(
                &format!("{}-{}.data/scripts/{}", normalized_name, info.package_version, exe),
                &exe_content,
                &mut record_entries,
            );
        }
    }

    // 4. Generate .dist-info directory
    let dist_info_dir_name = format!("{}-{}.dist-info", normalized_name, info.package_version);
    let dist_info_dir = temp_dir.join(&dist_info_dir_name);
    std::fs::create_dir_all(&dist_info_dir)?;

    // 5. Add METADATA file
    let metadata_content = generate_metadata(info);
    let metadata_path = dist_info_dir.join("METADATA");
    LocalAsset::write_new_all(&metadata_content, &metadata_path)?;
    add_file_to_record(
        &format!("{}/METADATA", dist_info_dir_name),
        metadata_content.as_bytes(),
        &mut record_entries,
    );

    // 6. Add WHEEL file
    let wheel_content = generate_wheel_file(info);
    let wheel_file_path = dist_info_dir.join("WHEEL");
    LocalAsset::write_new_all(&wheel_content, &wheel_file_path)?;
    add_file_to_record(
        &format!("{}/WHEEL", dist_info_dir_name),
        wheel_content.as_bytes(),
        &mut record_entries,
    );

    // 7. Add RECORD file (must be last, without hash for itself)
    let record_content = generate_record(&record_entries, &dist_info_dir_name);
    let record_path = dist_info_dir.join("RECORD");
    LocalAsset::write_new_all(&record_content, &record_path)?;

    // 8. Create ZIP archive using axoasset
    LocalAsset::zip_dir(&temp_dir, &wheel_path, None::<&Utf8PathBuf>)?;

    // 9. Clean up temporary directory
    std::fs::remove_dir_all(&temp_dir)?;

    info!("Generated Python wheel: {}", wheel_path);
    Ok(())
}

fn add_file_to_record(path: &str, content: &[u8], record_entries: &mut Vec<RecordEntry>) {
    // Calculate SHA256 hash
    let mut hasher = Sha256::new();
    hasher.update(content);
    let hash = format!("{:x}", hasher.finalize());

    record_entries.push(RecordEntry {
        path: path.to_string(),
        hash,
        size: content.len(),
    });
}


fn generate_metadata(info: &PypiWheelInstallerInfo) -> String {
    let mut metadata = String::new();

    // Required fields (PEP 566)
    metadata.push_str("Metadata-Version: 2.1\n");
    metadata.push_str(&format!("Name: {}\n", info.package_name));
    metadata.push_str(&format!("Version: {}\n", info.package_version));

    // Optional fields
    if let Some(desc) = &info.package_desc {
        metadata.push_str(&format!("Summary: {}\n", desc));
    } else {
        metadata.push_str(&format!("Summary: Binary distribution of {}\n", info.package_name));
    }

    if let Some(url) = &info.homepage_url {
        metadata.push_str(&format!("Home-page: {}\n", url));
    }

    if !info.package_authors.is_empty() {
        let author = info.package_authors.join(", ");
        metadata.push_str(&format!("Author: {}\n", author));
    }

    if let Some(license) = &info.package_license {
        metadata.push_str(&format!("License: {}\n", license));
    }

    // Classifiers for binary distribution
    metadata.push_str("Classifier: Development Status :: 4 - Beta\n");
    metadata.push_str("Classifier: Intended Audience :: Developers\n");
    metadata.push_str("Classifier: Topic :: Software Development :: Build Tools\n");

    // Python version requirement (minimal, just for pip compatibility)
    metadata.push_str("Requires-Python: >=3.8\n");

    metadata
}

fn generate_wheel_file(info: &PypiWheelInstallerInfo) -> String {
    let platform_tag = platform_tag_from_triple(info.target_triple.as_str());

    let mut wheel = String::new();
    wheel.push_str("Wheel-Version: 1.0\n");
    wheel.push_str(&format!(
        "Generator: cargo-dist {}\n",
        env!("CARGO_PKG_VERSION")
    ));
    wheel.push_str("Root-Is-Purelib: false\n");
    wheel.push_str(&format!("Tag: py3-none-{}\n", platform_tag));

    wheel
}

fn generate_record(entries: &[RecordEntry], dist_info_dir: &str) -> String {
    let mut record = String::new();

    for entry in entries {
        record.push_str(&format!(
            "{},sha256={},{}\n",
            entry.path, entry.hash, entry.size
        ));
    }

    // RECORD file itself has no hash
    record.push_str(&format!("{}/RECORD,,\n", dist_info_dir));

    record
}

fn wheel_filename(info: &PypiWheelInstallerInfo) -> String {
    let normalized_name = info.package_name.replace('-', "_");
    let platform_tag = platform_tag_from_triple(info.target_triple.as_str());
    format!(
        "{}-{}-py3-none-{}.whl",
        normalized_name, info.package_version, platform_tag
    )
}

fn platform_tag_from_triple(triple: &str) -> String {
    // Map Rust target triples to wheel platform tags
    match triple {
        // Linux x86_64
        "x86_64-unknown-linux-gnu" => "manylinux_2_17_x86_64".to_string(),
        "x86_64-unknown-linux-musl" => "musllinux_1_1_x86_64".to_string(),
        // Linux aarch64
        "aarch64-unknown-linux-gnu" => "manylinux_2_17_aarch64".to_string(),
        "aarch64-unknown-linux-musl" => "musllinux_1_1_aarch64".to_string(),
        // macOS x86_64
        "x86_64-apple-darwin" => "macosx_10_12_x86_64".to_string(),
        // macOS aarch64 (Apple Silicon)
        "aarch64-apple-darwin" => "macosx_11_0_arm64".to_string(),
        // Windows x86_64
        "x86_64-pc-windows-msvc" | "x86_64-pc-windows-gnu" => "win_amd64".to_string(),
        // Windows i686
        "i686-pc-windows-msvc" | "i686-pc-windows-gnu" => "win32".to_string(),
        // Windows aarch64
        "aarch64-pc-windows-msvc" => "win_arm64".to_string(),
        // Fallback for unknown platforms
        _ => {
            warn!(
                "Unknown platform triple for wheel: {}, using generic tag",
                triple
            );
            "unknown".to_string()
        }
    }
}
