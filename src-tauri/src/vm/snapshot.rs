use crate::vm::qemu::qmp_client::QmpClient;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotInfo {
    pub tag: String,
    pub description: String,
    pub created_at: u64,
}

pub struct SnapshotManager {
    vm_dir: PathBuf,
}

impl SnapshotManager {
    pub fn new(vm_dir: PathBuf) -> Self { Self { vm_dir } }

    fn path(&self) -> PathBuf { self.vm_dir.join("snapshots.toml") }

    pub fn list(&self) -> anyhow::Result<Vec<SnapshotInfo>> {
        let p = self.path();
        if p.exists() { Ok(toml::from_str(&std::fs::read_to_string(p)?)?) }
        else { Ok(vec![]) }
    }

    pub async fn create(&self, qmp: &mut QmpClient, tag: &str, desc: &str) -> anyhow::Result<SnapshotInfo> {
        qmp.command("savevm", serde_json::json!({"name": tag})).await?;
        let info = SnapshotInfo {
            tag: tag.to_string(), description: desc.to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
        };
        let mut snapshots = self.list()?;
        snapshots.retain(|s| s.tag != tag);
        snapshots.push(info.clone());
        std::fs::write(self.path(), toml::to_string_pretty(&snapshots)?)?;
        Ok(info)
    }

    pub async fn restore(qmp: &mut QmpClient, tag: &str) -> anyhow::Result<()> {
        qmp.command("loadvm", serde_json::json!({"name": tag})).await?;
        Ok(())
    }

    pub async fn delete(&self, qmp: &mut QmpClient, tag: &str) -> anyhow::Result<()> {
        qmp.command("delvm", serde_json::json!({"name": tag})).await?;
        let mut snapshots = self.list()?;
        snapshots.retain(|s| s.tag != tag);
        std::fs::write(self.path(), toml::to_string_pretty(&snapshots)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn snapshot_serialization() {
        let info = SnapshotInfo { tag: "boot".into(), description: "clean".into(), created_at: 1_700_000_000 };
        let toml = toml::to_string_pretty(&info).unwrap();
        let decoded: SnapshotInfo = toml::from_str(&toml).unwrap();
        assert_eq!(decoded.tag, "boot");
    }

    #[test]
    fn manager_list_empty_for_new_dir() {
        let dir = std::env::temp_dir().join("pds-test-snapshots");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mgr = SnapshotManager::new(dir.clone());
        assert!(mgr.list().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
