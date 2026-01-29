//! pypi_wheel installer config

use super::*;

/// Options for pypi_wheel installer (~raw config file contents)
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PypiWheelInstallerLayer {
    /// Common options
    #[serde(flatten)]
    pub common: CommonInstallerLayer,
}

/// Options for pypi_wheel installer (final)
#[derive(Debug, Default, Clone)]
pub struct PypiWheelInstallerConfig {
    /// Common options
    pub common: CommonInstallerConfig,
}

impl PypiWheelInstallerConfig {
    /// Get defaults for the given package
    pub fn defaults_for_package(
        _workspaces: &WorkspaceGraph,
        _pkg_idx: PackageIdx,
        common: &CommonInstallerConfig,
    ) -> Self {
        Self {
            common: common.clone(),
        }
    }
}

impl ApplyLayer for PypiWheelInstallerConfig {
    type Layer = PypiWheelInstallerLayer;
    fn apply_layer(&mut self, Self::Layer { common }: Self::Layer) {
        self.common.apply_layer(common);
    }
}
impl ApplyLayer for PypiWheelInstallerLayer {
    type Layer = PypiWheelInstallerLayer;
    fn apply_layer(&mut self, Self::Layer { common }: Self::Layer) {
        self.common.apply_layer(common);
    }
}

impl std::ops::Deref for PypiWheelInstallerConfig {
    type Target = CommonInstallerConfig;
    fn deref(&self) -> &Self::Target {
        &self.common
    }
}
