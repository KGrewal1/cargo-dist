//! pypi publisher config

use super::*;

/// Options for PyPI publishes (raw from file)
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PypiPublisherLayer {
    /// Common options
    pub common: CommonPublisherLayer,
    /// Use TestPyPI instead of PyPI (default: false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_pypi: Option<bool>,
    /// Enable PEP 740 attestations (default: true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attestations: Option<bool>,
    /// Custom index URL (overrides test_pypi)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_url: Option<String>,
}

/// Options for PyPI publishes (final)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PypiPublisherConfig {
    /// Common options
    pub common: CommonPublisherConfig,
    /// Use TestPyPI
    pub test_pypi: bool,
    /// Enable attestations
    pub attestations: bool,
    /// Custom index URL
    pub index_url: Option<String>,
}

impl PypiPublisherConfig {
    /// Get defaults for the given package
    pub fn defaults_for_package(
        _workspaces: &WorkspaceGraph,
        _pkg_idx: PackageIdx,
        common: &CommonPublisherConfig,
    ) -> Self {
        Self {
            common: common.clone(),
            test_pypi: false,
            attestations: true,
            index_url: None,
        }
    }
}

impl ApplyLayer for PypiPublisherConfig {
    type Layer = PypiPublisherLayer;
    fn apply_layer(
        &mut self,
        Self::Layer {
            common,
            test_pypi,
            attestations,
            index_url,
        }: Self::Layer,
    ) {
        self.common.apply_layer(common);
        self.test_pypi.apply_val(test_pypi);
        self.attestations.apply_val(attestations);
        self.index_url.apply_opt(index_url);
    }
}

impl ApplyLayer for PypiPublisherLayer {
    type Layer = PypiPublisherLayer;
    fn apply_layer(
        &mut self,
        Self::Layer {
            common,
            test_pypi,
            attestations,
            index_url,
        }: Self::Layer,
    ) {
        self.common.apply_layer(common);
        self.test_pypi.apply_opt(test_pypi);
        self.attestations.apply_opt(attestations);
        self.index_url.apply_opt(index_url);
    }
}

impl std::ops::Deref for PypiPublisherConfig {
    type Target = CommonPublisherConfig;
    fn deref(&self) -> &Self::Target {
        &self.common
    }
}
