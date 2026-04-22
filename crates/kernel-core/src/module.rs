use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ModuleKind {
    Provider,
    Tool,
    Memory,
    Policy,
    Channel,
    Observer,
    WebUi,
    GatewayWs,
    Browser,
    McpBridge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleManifest {
    pub name: String,
    pub version: String,
    pub kind: ModuleKind,
    pub capabilities: BTreeSet<String>,
    pub source: ModuleSource,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModuleSource {
    Builtin,
    Wasm {
        path: String,
    },
    Git {
        repo: String,
        rev: String,
        subdir: Option<String>,
    },
    LocalPath {
        path: String,
    },
}

#[derive(Debug, Default, Clone)]
pub struct ModuleRegistry {
    modules: BTreeMap<String, ModuleManifest>,
}

#[derive(Debug, Error)]
pub enum ModuleRegistryError {
    #[error("module already exists: {0}")]
    AlreadyExists(String),
    #[error("module not found: {0}")]
    NotFound(String),
}

impl ModuleRegistry {
    pub fn install(&mut self, manifest: ModuleManifest) -> Result<(), ModuleRegistryError> {
        if self.modules.contains_key(&manifest.name) {
            return Err(ModuleRegistryError::AlreadyExists(manifest.name));
        }
        self.modules.insert(manifest.name.clone(), manifest);
        Ok(())
    }

    pub fn enable(&mut self, name: &str) -> Result<(), ModuleRegistryError> {
        let module = self
            .modules
            .get_mut(name)
            .ok_or_else(|| ModuleRegistryError::NotFound(name.to_string()))?;
        module.enabled = true;
        Ok(())
    }

    pub fn disable(&mut self, name: &str) -> Result<(), ModuleRegistryError> {
        let module = self
            .modules
            .get_mut(name)
            .ok_or_else(|| ModuleRegistryError::NotFound(name.to_string()))?;
        module.enabled = false;
        Ok(())
    }

    pub fn uninstall(&mut self, name: &str) -> Result<ModuleManifest, ModuleRegistryError> {
        self.modules
            .remove(name)
            .ok_or_else(|| ModuleRegistryError::NotFound(name.to_string()))
    }

    pub fn list_by_kind(&self, kind: ModuleKind) -> Vec<&ModuleManifest> {
        self.modules
            .values()
            .filter(|module| module.kind == kind)
            .collect()
    }

    pub fn enabled_modules(&self) -> Vec<&ModuleManifest> {
        self.modules
            .values()
            .filter(|module| module.enabled)
            .collect()
    }

    pub fn has_capability(&self, capability: &str) -> bool {
        self.modules
            .values()
            .any(|module| module.enabled && module.capabilities.contains(capability))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installs_and_queries_modules() {
        let mut registry = ModuleRegistry::default();

        let mut caps = BTreeSet::new();
        caps.insert("tool.http".to_string());
        registry
            .install(ModuleManifest {
                name: "browser-module".to_string(),
                version: "0.1.0".to_string(),
                kind: ModuleKind::Browser,
                capabilities: caps,
                source: ModuleSource::Builtin,
                enabled: true,
            })
            .unwrap();

        assert_eq!(registry.list_by_kind(ModuleKind::Browser).len(), 1);
        assert!(registry.has_capability("tool.http"));

        registry.disable("browser-module").unwrap();
        assert!(!registry.has_capability("tool.http"));
    }
}
