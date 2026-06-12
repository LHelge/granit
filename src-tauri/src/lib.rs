mod agent;
mod cave;
mod commands;
mod markdown;

use commands::*;
use granit_types::AppConfig;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = AppConfig::default();

    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Debug)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState::new(config))
        .manage(UpdateCheckGuard::default())
        .setup(|app| {
            spawn_startup_update_check(app);
            restore_active_cave(app)
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            get_app_metadata,
            save_config,
            save_sidebar_state,
            list_system_fonts,
            open_cave,
            create_note,
            create_template,
            create_folder,
            delete_folder,
            move_note,
            move_folder,
            list_notes,
            list_anchors,
            list_templates,
            search_content,
            list_folders,
            read_note,
            read_template,
            open_daily_note,
            open_daily_note_for_date,
            save_note,
            save_template,
            delete_note,
            delete_template,
            rename_note,
            rename_template,
            rename_folder,
            update_note,
            update_template,
            render_note,
            render_template,
            render_markdown,
            set_active_note,
            list_todos,
            list_tags,
            toggle_todo,
            toggle_todo_by_index,
            list_providers,
            select_provider,
            list_models,
            select_model,
            select_mode,
            send_message,
            clear_chat,
            list_tools,
            get_pending_release_notes,
            acknowledge_release_notes,
            check_for_updates,
            restart_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
