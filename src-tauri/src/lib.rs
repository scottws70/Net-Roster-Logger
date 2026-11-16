use std::fs;
use std::path::Path;

mod mail;
mod qrz;
mod share;
mod store;
mod weather;

/// Saves a text file into a chosen folder. If a file with `filename` already
/// exists there, `alt_filename` (the same name with a time stamp) is used instead,
/// so an earlier backup is never overwritten. Returns the name actually used.
#[tauri::command]
fn save_text_file(
    dir: String,
    filename: String,
    alt_filename: String,
    contents: String,
) -> Result<String, String> {
    let dir_path = Path::new(&dir);
    if !dir_path.is_dir() {
        return Err(format!("Folder not found: {}", dir));
    }

    // Keep only the final name component, so a bad name can't escape the folder.
    let pick = |name: &str| -> Result<String, String> {
        Path::new(name)
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.to_string())
            .ok_or_else(|| "Invalid file name".to_string())
    };

    let mut final_name = pick(&filename)?;
    if dir_path.join(&final_name).exists() {
        final_name = pick(&alt_filename)?;
    }

    fs::write(dir_path.join(&final_name), contents).map_err(|e| e.to_string())?;
    Ok(final_name)
}

/// Writes a text file to an exact path (used after a "Save As" dialog).
#[tauri::command]
fn write_text_file(path: String, contents: String) -> Result<(), String> {
    fs::write(&path, contents).map_err(|e| e.to_string())
}

/// One CSV file read from a folder, for the Net Manager's Summary.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CsvFile {
    name: String,
    /// Last modified time, in milliseconds since 1970
    modified: u64,
    contents: String,
}

/// Reads every .csv file directly inside `dir` (not in subfolders), skipping very large
/// files and ones that aren't text.
#[tauri::command]
fn read_csv_folder(dir: String) -> Result<Vec<CsvFile>, String> {
    let entries = fs::read_dir(&dir).map_err(|e| format!("Couldn't open {}: {}", dir, e))?;
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let is_csv = path
            .extension()
            .and_then(|x| x.to_str())
            .is_some_and(|x| x.eq_ignore_ascii_case("csv"));
        let Ok(meta) = entry.metadata() else { continue };
        if !is_csv || !meta.is_file() || meta.len() > 5_000_000 {
            continue;
        }
        let Ok(contents) = fs::read_to_string(&path) else { continue };
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let name = entry.file_name().to_string_lossy().into_owned();
        files.push(CsvFile { name, modified, contents });
    }
    files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(files)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    // A second copy of the app would overwrite the first one's saves: bring the open
    // window forward instead. (Not in development builds, which keep their own nets.)
    #[cfg(all(desktop, not(debug_assertions)))]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        use tauri::Manager;
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.unminimize();
            let _ = w.show();
            let _ = w.set_focus();
        }
    }));
    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .manage(qrz::QrzSession::default())
        .manage(share::ShareState::default())
        .manage(store::StoreState::default())
        .setup(|app| {
            // Sharing (serving this computer's nets to other devices) and in-app
            // updates are desktop-only
            #[cfg(desktop)]
            {
                share::init(app.handle());
                app.handle().plugin(tauri_plugin_updater::Builder::new().build())?;
                app.handle().plugin(tauri_plugin_process::init())?;
            }
            #[cfg(not(desktop))]
            let _ = app;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            save_text_file,
            write_text_file,
            read_csv_folder,
            mail::mail_status,
            mail::mail_save_settings,
            mail::mail_clear_settings,
            mail::mail_test,
            mail::mail_send,
            qrz::qrz_status,
            qrz::qrz_save_credentials,
            qrz::qrz_clear_credentials,
            qrz::qrz_test_login,
            qrz::qrz_photos_dir,
            qrz::qrz_saved_photos,
            qrz::qrz_lookup,
            weather::weather_has_wu_key,
            weather::weather_save_wu_key,
            weather::weather_fetch,
            share::share_status,
            share::share_set,
            share::share_new_code,
            share::share_publish,
            share::share_probe,
            store::store_load,
            store::store_save,
            store::store_migrate,
            store::store_snapshots,
            store::store_snapshot_read,
            store::store_keep_before_restore,
            store::store_folder,
            store::store_open_folder
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
