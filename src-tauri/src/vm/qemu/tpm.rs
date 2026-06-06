use std::path::PathBuf;
use tokio::process::{Child, Command};

/// Manages a swtpm (Software TPM 2.0) subprocess for Windows 11 ARM.
pub struct Swtpm {
    child: Option<Child>,
    socket_path: PathBuf,
    state_dir: PathBuf,
}

impl Swtpm {
    pub fn new(vm_id: &str) -> Self {
        let state_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("parallels-desktop-rs")
            .join("tpm")
            .join(vm_id);
        let socket_path = state_dir.join("swtpm.sock");
        Self { child: None, socket_path, state_dir }
    }

    pub fn socket_path(&self) -> &PathBuf { &self.socket_path }

    pub async fn start(&mut self) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.state_dir)?;
        let child = Command::new("swtpm")
            .arg("socket")
            .arg("--tpm2")
            .arg("--tpmstate").arg(format!("dir={}", self.state_dir.display()))
            .arg("--ctrl").arg(format!("type=unixio,path={}", self.socket_path.display()))
            .arg("--log").arg("level=1")
            .kill_on_drop(true)
            .spawn()?;
        self.child = Some(child);
        for _ in 0..10 {
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            if self.socket_path.exists() { return Ok(()); }
        }
        anyhow::bail!("swtpm socket did not appear");
    }

    pub async fn stop(&mut self) -> anyhow::Result<()> {
        if let Some(mut child) = self.child.take() {
            child.start_kill()?;
            child.wait().await?;
        }
        Ok(())
    }
}

/// Generate QEMU TPM device arguments.
pub fn tpm_qemu_args(socket_path: &str) -> Vec<String> {
    vec![
        "-tpmdev".into(), format!("emulator,id=tpm0,chardev=chrtpm"),
        "-chardev".into(), format!("socket,id=chrtpm,path={socket_path}"),
        "-device".into(), "tpm-tis-device,tpmdev=tpm0".into(),
    ]
}
