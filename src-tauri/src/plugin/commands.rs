use crate::plugin::events::{VmErrorEvent, VmStatusEvent};
use crate::plugin::state::AppState;
use crate::vm::config::VmConfig;
use crate::vm::manager::VmManager;
use crate::vm::snapshot::SnapshotManager;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub async fn create_vm(config: VmConfig) -> Result<VmConfig, String> {
    config.save().map_err(|e| format!("Failed to save VM config: {e}"))?;
    Ok(config)
}

#[tauri::command]
pub async fn list_vms() -> Result<Vec<VmConfig>, String> {
    VmConfig::list_all().map_err(|e| format!("Failed to list VMs: {e}"))
}

#[tauri::command]
pub async fn delete_vm(vm_id: String) -> Result<(), String> {
    let vm_dir = VmConfig::list_all()
        .map_err(|e| format!("{e}"))?
        .into_iter()
        .find(|c| c.id == vm_id)
        .map(|c| c.vm_dir())
        .ok_or_else(|| format!("VM '{vm_id}' not found"))?;
    std::fs::remove_dir_all(&vm_dir)
        .map_err(|e| format!("Failed to delete VM data: {e}"))
}

#[tauri::command]
pub async fn start_vm(
    app: AppHandle,
    state: State<'_, AppState>,
    vm_id: String,
) -> Result<(), String> {
    let vm_dir = VmConfig::list_all()
        .map_err(|e| format!("{e}"))?
        .into_iter()
        .find(|c| c.id == vm_id)
        .map(|c| c.vm_dir())
        .ok_or_else(|| format!("VM '{vm_id}' not found"))?;

    let config = VmConfig::load(&vm_dir).map_err(|e| format!("Failed to load config: {e}"))?;
    let mut manager = VmManager::new(config.clone());

    manager.start(app.clone()).await.map_err(|e| {
        let msg = format!("Failed to start VM: {e}");
        let _ = app.emit("vm:error", VmErrorEvent::new(&vm_id, &msg));
        msg
    })?;

    let _ = app.emit("vm:status", VmStatusEvent::new(&vm_id, "running"));
    state.put(vm_id.clone(), manager).await;
    Ok(())
}

#[tauri::command]
pub async fn stop_vm(
    app: AppHandle,
    state: State<'_, AppState>,
    vm_id: String,
) -> Result<(), String> {
    if let Some(mut manager) = state.get(&vm_id).await {
        manager.stop().await.map_err(|e| {
            let _ = app.emit("vm:error", VmErrorEvent::new(&vm_id, e.to_string()));
            e.to_string()
        })?;
        let _ = app.emit("vm:status", VmStatusEvent::new(&vm_id, "stopped"));
        Ok(())
    } else {
        Err(format!("VM '{vm_id}' is not running"))
    }
}

#[tauri::command]
pub async fn get_vm_status(state: State<'_, AppState>, vm_id: String) -> Result<String, String> {
    if let Some(manager) = state.get(&vm_id).await {
        let status = format!("{:?}", manager.state().await);
        state.put(vm_id, manager).await;
        Ok(status)
    } else {
        Ok("stopped".into())
    }
}

// -- Snapshot commands --

#[tauri::command]
pub async fn snapshot_list(vm_id: String) -> Result<Vec<crate::vm::snapshot::SnapshotInfo>, String> {
    let vm_dir = VmConfig::list_all()
        .map_err(|e| format!("{e}"))?
        .into_iter()
        .find(|c| c.id == vm_id)
        .map(|c| c.vm_dir())
        .ok_or_else(|| format!("VM '{vm_id}' not found"))?;
    SnapshotManager::new(vm_dir).list().map_err(|e| format!("{e}"))
}

#[tauri::command]
pub async fn snapshot_create(
    state: State<'_, AppState>,
    vm_id: String,
    tag: String,
    description: String,
) -> Result<crate::vm::snapshot::SnapshotInfo, String> {
    if let Some(mut manager) = state.get(&vm_id).await {
        let vm_dir = manager.config.vm_dir();
        let mgr = SnapshotManager::new(vm_dir);
        if let Some(ref mut qmp) = manager.qmp() {
            let info = mgr.create(qmp, &tag, &description).await.map_err(|e| format!("{e}"))?;
            state.put(vm_id, manager).await;
            return Ok(info);
        }
        state.put(vm_id, manager).await;
        Err("VM is running but QMP is not connected".into())
    } else {
        Err(format!("VM '{vm_id}' is not running"))
    }
}

#[tauri::command]
pub async fn snapshot_restore(
    state: State<'_, AppState>,
    vm_id: String,
    tag: String,
) -> Result<(), String> {
    if let Some(mut manager) = state.get(&vm_id).await {
        if let Some(ref mut qmp) = manager.qmp() {
            SnapshotManager::restore(qmp, &tag).await.map_err(|e| format!("{e}"))?;
        }
        state.put(vm_id, manager).await;
        Ok(())
    } else {
        Err(format!("VM '{vm_id}' is not running"))
    }
}

#[tauri::command]
pub async fn snapshot_delete(
    state: State<'_, AppState>,
    vm_id: String,
    tag: String,
) -> Result<(), String> {
    if let Some(mut manager) = state.get(&vm_id).await {
        let vm_dir = manager.config.vm_dir();
        let mgr = SnapshotManager::new(vm_dir);
        if let Some(ref mut qmp) = manager.qmp() {
            mgr.delete(qmp, &tag).await.map_err(|e| format!("{e}"))?;
        }
        state.put(vm_id, manager).await;
        Ok(())
    } else {
        Err(format!("VM '{vm_id}' is not running"))
    }
}

// -- Suspend/Resume --

#[tauri::command]
pub async fn suspend_vm(
    app: AppHandle,
    state: State<'_, AppState>,
    vm_id: String,
) -> Result<(), String> {
    if let Some(mut manager) = state.get(&vm_id).await {
        let state_file = manager.config.vm_dir().join("state.mig").display().to_string();
        manager.suspend(&state_file).await.map_err(|e| format!("{e}"))?;
        manager.stop().await.map_err(|e| format!("{e}"))?;
        let _ = app.emit("vm:status", VmStatusEvent::new(&vm_id, "suspended"));
        Ok(())
    } else {
        Err(format!("VM '{vm_id}' is not running"))
    }
}

#[tauri::command]
pub async fn resume_vm(
    app: AppHandle,
    state: State<'_, AppState>,
    vm_id: String,
) -> Result<(), String> {
    let _ = app.emit("vm:status", VmStatusEvent::new(&vm_id, "resuming"));
    start_vm(app, state, vm_id).await
}
