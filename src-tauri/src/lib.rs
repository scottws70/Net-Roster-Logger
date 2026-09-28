use std::fs;
use std::path::Path;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![save_text_file, write_text_file])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
