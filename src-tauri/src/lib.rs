mod platform;
mod plugin;
mod vm;

use plugin::commands;
use plugin::state::AppState;
use tracing_subscriber::EnvFilter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tauri::Builder::default()
        .setup(|app| {
            // Platform validation
            platform::macos::check_apple_silicon()
                .expect("This application requires Apple Silicon");
            platform::macos::check_hv_support()
                .expect("Hypervisor.framework is not available");

            // Verify Metal is available for VM display
            #[cfg(target_os = "macos")]
            if let Some(device) = platform::macos::metal_view::default_device() {
                tracing::info!("Metal device: {}", device.name());
            } else {
                tracing::warn!("No Metal device found; VM display will use serial console only");
            }

            tracing::info!("Parallels Desktop RS starting up");

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            Ok(())
        })
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::create_vm,
            commands::list_vms,
            commands::delete_vm,
            commands::start_vm,
            commands::stop_vm,
            commands::get_vm_status,
            commands::snapshot_list,
            commands::snapshot_create,
            commands::snapshot_restore,
            commands::snapshot_delete,
            commands::suspend_vm,
            commands::resume_vm,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
