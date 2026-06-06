use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// VM configuration persisted to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    #[serde(default = "new_vm_id")]
    pub id: String,
    pub name: String,
    pub cpu_count: u32,
    pub memory_mb: u64,
    pub disk_path: PathBuf,
    pub iso_path: Option<PathBuf>,
    #[serde(default)]
    pub os_type: OsType,
    #[serde(default)]
    pub display: DisplayMode,
    #[serde(default)]
    pub shared_folders: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum OsType {
    #[default]
    Linux,
    Windows,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum DisplayMode {
    #[default]
    Serial,
    Graphical,
}

impl VmConfig {
    pub fn new(name: impl Into<String>, disk_path: impl Into<PathBuf>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            cpu_count: 4,
            memory_mb: 4096,
            disk_path: disk_path.into(),
            iso_path: None,
            os_type: OsType::Linux,
            display: DisplayMode::Serial,
            shared_folders: Vec::new(),
        }
    }

    pub fn vm_dir(&self) -> PathBuf {
        data_dir().join(&self.id)
    }

    pub fn qmp_socket_path(&self) -> PathBuf {
        // macOS Unix socket paths are limited to 104 bytes (sockaddr_un.sun_path).
        // Use /tmp with a short id prefix to stay under the limit.
        let short = &self.id[..self.id.len().min(8)];
        PathBuf::from(format!("/tmp/pds-{short}.sock"))
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let dir = self.vm_dir();
        std::fs::create_dir_all(&dir)?;
        let toml = toml::to_string_pretty(self)?;
        std::fs::write(dir.join("config.toml"), toml)?;
        Ok(())
    }

    pub fn load(vm_dir: &PathBuf) -> anyhow::Result<Self> {
        let toml = std::fs::read_to_string(vm_dir.join("config.toml"))?;
        Ok(toml::from_str(&toml)?)
    }

    pub fn list_all() -> anyhow::Result<Vec<Self>> {
        let data = data_dir();
        if !data.exists() {
            return Ok(vec![]);
        }
        let mut configs = Vec::new();
        for entry in std::fs::read_dir(&data)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let config_path = entry.path().join("config.toml");
                if config_path.exists() {
                    if let Ok(cfg) = Self::load(&entry.path()) {
                        configs.push(cfg);
                    }
                }
            }
        }
        Ok(configs)
    }
}

pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("parallels-desktop-rs")
        .join("vms")
}

fn new_vm_id() -> String {
    Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_config_has_valid_id() {
        let config = VmConfig::new("test-vm", "/tmp/test.qcow2");
        assert!(!config.id.is_empty());
        assert_eq!(config.name, "test-vm");
        assert_eq!(config.cpu_count, 4);
    }

    #[test]
    fn config_serialization_roundtrip() {
        let config = VmConfig::new("roundtrip", "/tmp/disk.qcow2");
        let toml = toml::to_string_pretty(&config).unwrap();
        let decoded: VmConfig = toml::from_str(&toml).unwrap();
        assert_eq!(config.id, decoded.id);
        assert_eq!(config.cpu_count, decoded.cpu_count);
    }
}
