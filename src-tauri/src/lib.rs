// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod tray;

use prometheus_substrate::{node, paths};
use tauri::Manager;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .setup(|app| {
            let handle = app.handle().clone();
            // The node's startup is async (it awaits a socket bind and the
            // authority open); `setup` itself is sync, so the work happens on
            // Tauri's async runtime and the tray is installed once a health
            // subscription — or the knowledge that there isn't one — exists.
            tauri::async_runtime::spawn(async move {
                match node::spawn(&paths::skills_dir()).await {
                    Ok((node_handle, readiness)) => {
                        tracing::info!(?readiness, "companion node started");
                        let health = node_handle.subscribe_health();
                        // The handle is kept in Tauri-managed state so it is
                        // not dropped. Drop requests cancellation; explicit
                        // async shutdown is needed to wait for listener drain.
                        handle.manage(node_handle);
                        match health {
                            Some(health) => {
                                if let Err(error) = tray::install(&handle, health) {
                                    tracing::error!(%error, "failed to install tray icon");
                                }
                            }
                            None => {
                                tracing::warn!(
                                    "no service-health subscription available; tray will not \
                                     be installed until supervision is attached \
                                     (set PROMETHEUS_PACK_ROOT)"
                                );
                            }
                        }
                    }
                    Err(error) => {
                        // Not a panic: the setup hook already returned Ok, and
                        // a windowed app that failed to start its node should
                        // still show a window an operator can read logs from,
                        // rather than the whole process refusing to launch.
                        tracing::error!(%error, "failed to start companion node");
                    }
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
