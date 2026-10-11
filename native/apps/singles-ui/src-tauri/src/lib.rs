#[cfg(feature = "desktop")]
mod commands;
#[cfg(any(feature = "desktop", test))]
mod job_owner;
#[cfg(any(feature = "desktop", test))]
mod profile;

#[cfg(feature = "desktop")]
pub fn run() {
    let builder = tauri::Builder::default()
        .manage(job_owner::Host::with_model_directory(
            std::env::current_exe()
                .expect("application path")
                .parent()
                .expect("application directory")
                .join("models")
                .join("original"),
        ))
        .plugin(tauri_plugin_opener::init());
    #[cfg(any(windows, target_os = "linux"))]
    let builder = builder.plugin(tauri_plugin_dialog::init());
    builder
        .setup(|app| {
            use tauri::Manager;
            app.manage(profile::ProfileStore::new(
                app.path().app_local_data_dir()?.join("profile-v1"),
            ));
            let window = tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("מסדר הסינגלים")
            .inner_size(1000.0, 650.0)
            .min_inner_size(340.0, 480.0)
            .devtools(false)
            .on_page_load({
                let host = app.state::<job_owner::Host>().inner().clone();
                move |_, payload| {
                    if matches!(payload.event(), tauri::webview::PageLoadEvent::Started) {
                        host.cancel_active();
                    }
                }
            })
            .on_navigation(|url| {
                (url.scheme() == "tauri" && url.host_str() == Some("localhost"))
                    || (matches!(url.scheme(), "http" | "https")
                        && url.host_str() == Some("tauri.localhost"))
                    || (cfg!(debug_assertions)
                        && url.scheme() == "http"
                        && url.host_str() == Some("localhost")
                        && url.port() == Some(1420))
            })
            .build()?;
            window.on_window_event({
                let host = app.state::<job_owner::Host>().inner().clone();
                move |event| {
                    if matches!(
                        event,
                        tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed
                    ) {
                        host.cancel_active();
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::select_root,
            commands::resolve_typed_root,
            commands::start_preview,
            commands::cancel_preview,
            commands::job_snapshot,
            commands::get_plan_page,
            commands::get_settings,
            commands::get_model_status,
            commands::open_footer_link,
            commands::save_main_settings,
            commands::set_duet,
            commands::get_personal_list,
            commands::save_personal_list,
            commands::select_personal_import,
            commands::preview_personal_import,
            commands::confirm_personal_import,
            commands::export_personal_list,
            commands::get_first_run_status,
            commands::mark_first_run_seen,
        ])
        .run(tauri::generate_context!())
        .expect("run read-only Singles Sorter preview host");
}
