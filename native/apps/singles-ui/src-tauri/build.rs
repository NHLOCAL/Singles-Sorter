fn main() {
    #[cfg(feature = "desktop")]
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        #[cfg(feature = "desktop")]
        tauri_build::AppManifest::new().commands(&[
            "start_recovery_list",
            "recovery_snapshot",
            "cancel_recovery",
            "select_recovery_root",
            "select_recovery_parent",
            "start_recovery_inspection",
            "select_root",
            "resolve_typed_root",
            "start_preview",
            "cancel_preview",
            "job_snapshot",
            "get_plan_page",
            "get_settings",
            "get_model_status",
            "open_footer_link",
            "open_project_link",
            "save_main_settings",
            "set_duet",
            "get_personal_list",
            "save_personal_list",
            "select_personal_import",
            "preview_personal_import",
            "confirm_personal_import",
            "export_personal_list",
            "get_first_run_status",
            "mark_first_run_seen",
        ]),
    ))
    .expect("build restricted read-only host capabilities");
}
