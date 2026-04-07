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
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(AppState::new(config))
        .setup(|app| {
            restore_active_cave(app).map_err(std::io::Error::other)?;
            Ok(())
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
            list_templates,
            search_content,
            list_folders,
            read_note,
            read_template,
            open_daily_note,
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
            toggle_todo,
            toggle_todo_by_index,
            list_providers,
            select_provider,
            list_models,
            select_model,
            send_message,
            clear_chat,
            list_tools,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
