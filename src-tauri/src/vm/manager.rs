use crate::plugin::events::SerialOutputEvent;
use crate::vm::config::{runtime_dir, VmConfig};
use crate::vm::qemu::command_builder::build_qemu_args;
use crate::vm::qemu::qmp_client::QmpClient;
use std::sync::Arc;
use tauri::Emitter;
use tokio::io::AsyncBufReadExt;
use tokio::sync::{oneshot, Mutex};

#[derive(Debug, Clone, PartialEq)]
pub enum VmState {
    Stopped,
    Starting,
    Running { pid: u32 },
    Stopping,
    Error { message: String },
}

pub struct VmManager {
    pub config: VmConfig,
    state: Arc<Mutex<VmState>>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    qmp: Option<QmpClient>,
}

impl VmManager {
    pub fn new(config: VmConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(VmState::Stopped)),
            shutdown_tx: None,
            qmp: None,
        }
    }

    pub async fn state(&self) -> VmState {
        self.state.lock().await.clone()
    }

    pub fn qmp(&mut self) -> Option<&mut QmpClient> {
        self.qmp.as_mut()
    }

    /// Suspend VM: save full state to file via QMP migrate.
    pub async fn suspend(&mut self, state_file: &str) -> anyhow::Result<()> {
        if let Some(ref mut qmp) = self.qmp {
            qmp.command("migrate", serde_json::json!({"uri": format!("exec:cat > {state_file}")}))
                .await?;
            // Wait for migration to complete (simplified: just sleep)
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            tracing::info!("VM suspended to {state_file}");
        }
        Ok(())
    }

    /// Resume VM from saved state file.
    pub fn resume_args(&self, state_file: &str) -> Vec<String> {
        let mut args = build_qemu_args(&self.config);
        args.push("-incoming".into());
        args.push(format!("exec:cat {state_file}"));
        args
    }

    pub async fn start(&mut self, app: tauri::AppHandle) -> anyhow::Result<()> {
        let mut state = self.state.lock().await;
        if *state != VmState::Stopped {
            anyhow::bail!("VM is not stopped (state: {:?})", *state);
        }
        *state = VmState::Starting;

        let args = build_qemu_args(&self.config);
        std::fs::create_dir_all(runtime_dir())?;

        tracing::info!("Starting QEMU: {}", args.join(" "));

        let mut cmd = tokio::process::Command::new(&args[0]);
        cmd.args(&args[1..]);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        // stdio is piped above for serial output; stdin is null
        cmd.stdin(std::process::Stdio::null());

        let mut child = cmd.spawn().map_err(|e| {
            *state = VmState::Error {
                message: e.to_string(),
            };
            anyhow::anyhow!("Failed to spawn QEMU: {e}")
        })?;

        let pid = child.id().unwrap_or(0);

        // Capture stdout -> serial output events
        if let Some(stdout) = child.stdout.take() {
            let vm_id = self.config.id.clone();
            let app_handle = app.clone();
            tokio::spawn(async move {
                let reader = tokio::io::BufReader::new(stdout);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let _ = app_handle.emit(
                        "vm:serial",
                        SerialOutputEvent::new(&vm_id, &line),
                    );
                }
            });
        }

        // Capture stderr
        if let Some(stderr) = child.stderr.take() {
            let vm_id = self.config.id.clone();
            let app_handle = app.clone();
            tokio::spawn(async move {
                let reader = tokio::io::BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let _ = app_handle.emit(
                        "vm:serial",
                        SerialOutputEvent::new(&vm_id, format!("[stderr] {line}")),
                    );
                }
            });
        }

        // Connect QMP (retry until socket is ready)
        let qmp_path = self.config.qmp_socket_path();
        let mut qmp: Option<QmpClient> = None;
        let mut events_rx = None;
        for _ in 0..10 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            match QmpClient::connect(&qmp_path).await {
                Ok(client) => {
                    let mut c = client;
                    let rx = std::mem::replace(&mut c.events, tokio::sync::mpsc::unbounded_channel().1);
                    tracing::info!("QMP connected for VM {}", self.config.id);
                    events_rx = Some(rx);
                    qmp = Some(c);
                    break;
                }
                Err(e) => {
                    tracing::debug!("QMP connect retry: {e}");
                }
            }
        }

        self.qmp = qmp;

        // Spawn QMP event monitor for SHUTDOWN detection
        if let Some(mut rx) = events_rx {
            let state_mon = self.state.clone();
            let app_mon = app.clone();
            let vm_id_mon = self.config.id.clone();
            tokio::spawn(async move {
                while let Some(event) = rx.recv().await {
                    if event.event == "SHUTDOWN" {
                        tracing::info!("QMP SHUTDOWN event for VM {vm_id_mon}");
                        *state_mon.lock().await = VmState::Stopped;
                        let _ = app_mon.emit("vm:status",
                            crate::plugin::events::VmStatusEvent::new(&vm_id_mon, "stopped"));
                        break;
                    }
                }
            });
        }

        *state = VmState::Running { pid };

        // Background task: wait for QEMU exit
        let vm_id = self.config.id.clone();
        let state_clone = self.state.clone();
        let (tx, mut rx) = oneshot::channel::<()>();
        self.shutdown_tx = Some(tx);

        tokio::spawn(async move {
            tokio::select! {
                _ = &mut rx => {}
                status = child.wait() => {
                    let mut s = state_clone.lock().await;
                    match status {
                        Ok(exit) if exit.success() => {
                            tracing::info!("VM {vm_id} exited normally");
                        }
                        Ok(exit) => {
                            tracing::warn!("VM {vm_id} exited with code {:?}", exit.code());
                        }
                        Err(e) => {
                            tracing::error!("VM {vm_id} process error: {e}");
                            *s = VmState::Error { message: e.to_string() };
                        }
                    }
                    if !matches!(&*s, VmState::Error { .. }) {
                        *s = VmState::Stopped;
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn stop(&mut self) -> anyhow::Result<()> {
        let pid = {
            let mut state = self.state.lock().await;
            let pid = match &*state {
                VmState::Running { pid } => *pid,
                VmState::Stopped => return Ok(()),
                other => anyhow::bail!("Cannot stop VM in state: {other:?}"),
            };
            *state = VmState::Stopping;
            pid
        };

        // Attempt graceful shutdown via QMP
        if let Some(ref mut qmp) = self.qmp {
            let _ = qmp.system_powerdown().await;
            for _ in 0..20 {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                let s = self.state.lock().await;
                if *s == VmState::Stopped {
                    return Ok(());
                }
            }
        }

        // Force kill
        tracing::warn!("VM {} force-killing PID {pid}", self.config.id);
        unsafe {
            libc::kill(pid as i32, libc::SIGKILL);
            libc::waitpid(pid as i32, std::ptr::null_mut(), 0);
        }
        *self.state.lock().await = VmState::Stopped;
        Ok(())
    }
}
