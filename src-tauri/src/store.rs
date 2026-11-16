//! Where the desktop app keeps its nets: plain files in the app's data folder, one per
//! net, instead of the web view's own storage (which is small and can be wiped).
//!
//! ```text
//! <app data>/nets/index.json                    which net is open, and their order
//! <app data>/nets/<net id>.json                 one net: settings, roster, check-ins
//! <app data>/nets/snapshots/<net id>/<date>.json     the net as it was before that day's changes
//! <app data>/nets/snapshots/<net id>/before-restore.json   the net as it was before the last restore
//! <app data>/nets/before-1.45.json              the web view's data as it was when moved here
//! ```
//!
//! The page still works on the whole store at once: it hands over the full store on
//! every save, and only the nets that changed are rewritten. Every write goes to a
//! temporary file that is then renamed into place, so a crash can't leave half a file.

use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, State};

/// Days of snapshots kept for each net (days the net changed, not calendar days)
const KEEP_SNAPSHOTS: usize = 7;
/// A development build keeps its nets apart, so trying one out never touches real nets
const DIR_NAME: &str = if cfg!(debug_assertions) { "nets-dev" } else { "nets" };
const BEFORE_RESTORE: &str = "before-restore";
const PRE_MIGRATION_FILE: &str = "before-1.45.json";

/// What is on disk now, so a save rewrites only what changed.
#[derive(Default)]
pub struct StoreState(Mutex<Cache>);

#[derive(Default)]
struct Cache {
    nets: HashMap<String, Saved>,
    index: String,
}

struct Saved {
    /// The net exactly as written to its file
    full: String,
    /// The same without the roll-call position, which changes just by opening the Main
    /// tab each day: on its own it isn't worth a snapshot
    significant: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    id: String,
    /// What the unreadable file was renamed to (it is kept, never deleted)
    kept_as: String,
    /// The newest snapshot of this net, if there is one to fall back on
    newest_snapshot: Option<String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Loaded {
    /// False when nothing has been stored here yet (first run of a version with file storage)
    exists: bool,
    /// The whole store as JSON: {"version":2,"activeNetId":"…","nets":{…}}
    store: String,
    problems: Vec<Problem>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotInfo {
    /// "2026-11-16", or "before-restore"
    name: String,
    /// When the file was written, in milliseconds since 1970
    modified: u64,
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id != "index"
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// YYYY-MM-DD
fn valid_date(d: &str) -> bool {
    let b = d.as_bytes();
    b.len() == 10 && b.iter().enumerate().all(|(i, c)| if i == 4 || i == 7 { *c == b'-' } else { c.is_ascii_digit() })
}

fn canonical(net: &Value) -> Saved {
    let full = net.to_string();
    let mut trimmed = net.clone();
    if let Some(m) = trimmed.as_object_mut() {
        m.remove("mainCursor");
        m.remove("mainCursorDate");
    }
    Saved { full, significant: trimmed.to_string() }
}

/// Writes to a temporary file and renames it into place.
fn atomic_write(path: &Path, text: &str) -> Result<(), String> {
    let tmp = path.with_extension("json.tmp");
    let write = || -> std::io::Result<()> {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(text.as_bytes())?;
        f.sync_all()?;
        fs::rename(&tmp, path)
    };
    write().map_err(|e| format!("Couldn't write {}: {}", path.display(), e))
}

fn net_path(root: &Path, id: &str) -> PathBuf {
    root.join(format!("{id}.json"))
}

fn snapshot_dir(root: &Path, id: &str) -> PathBuf {
    root.join("snapshots").join(id)
}

/// The dated snapshots of a net, newest first.
fn snapshot_dates(root: &Path, id: &str) -> Vec<String> {
    let mut dates: Vec<String> = fs::read_dir(snapshot_dir(root, id))
        .map(|rd| {
            rd.flatten()
                .filter_map(|e| e.file_name().into_string().ok())
                .filter_map(|n| n.strip_suffix(".json").map(str::to_string))
                .filter(|n| valid_date(n))
                .collect()
        })
        .unwrap_or_default();
    dates.sort();
    dates.reverse();
    dates
}

/// Keeps the net's file as it is now as today's snapshot, unless today already has one
/// (the first one of the day is the state before that day's changes).
fn take_snapshot(root: &Path, id: &str, today: &str) -> Result<(), String> {
    let src = net_path(root, id);
    if !src.is_file() {
        return Ok(());
    }
    let dir = snapshot_dir(root, id);
    let dst = dir.join(format!("{today}.json"));
    if dst.exists() {
        return Ok(());
    }
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    fs::copy(&src, &dst).map_err(|e| format!("Couldn't save a snapshot: {e}"))?;
    for old in snapshot_dates(root, id).into_iter().skip(KEEP_SNAPSHOTS) {
        let _ = fs::remove_file(dir.join(format!("{old}.json")));
    }
    Ok(())
}

/// Writes the store's nets to their files: only those that changed, after a snapshot
/// when it's the first real change of the day. Files of nets no longer in the store
/// are removed (after a snapshot too). An empty or malformed store is refused, so a
/// fault in the page can never wipe the files.
fn save_to(root: &Path, cache: &mut Cache, store: &str, today: &str) -> Result<(), String> {
    if !valid_date(today) {
        return Err("Bad date.".into());
    }
    let v: Value = serde_json::from_str(store).map_err(|e| format!("That isn't Net Roster Logger data: {e}"))?;
    let nets = v["nets"].as_object().filter(|m| !m.is_empty()).ok_or("Refused to save a store with no nets in it.")?;
    for (id, net) in nets {
        if !valid_id(id) || !net.is_object() {
            return Err(format!("Refused to save a net with a bad name or contents ({id})."));
        }
    }
    fs::create_dir_all(root).map_err(|e| format!("Couldn't create {}: {}", root.display(), e))?;

    for (id, net) in nets {
        let now = canonical(net);
        let old = cache.nets.get(id);
        if old.is_some_and(|o| o.full == now.full) {
            continue;
        }
        if old.is_none_or(|o| o.significant != now.significant) {
            take_snapshot(root, id, today)?;
        }
        atomic_write(&net_path(root, id), &now.full)?;
        cache.nets.insert(id.clone(), now);
    }

    let gone: Vec<String> = cache.nets.keys().filter(|id| !nets.contains_key(*id)).cloned().collect();
    for id in gone {
        take_snapshot(root, &id, today)?;
        fs::remove_file(net_path(root, &id)).map_err(|e| e.to_string())?;
        cache.nets.remove(&id);
    }

    let order: Vec<&String> = nets.keys().collect();
    let index = json!({ "version": 1, "activeNetId": v["activeNetId"].as_str().unwrap_or(""), "order": order }).to_string();
    if index != cache.index {
        atomic_write(&root.join("index.json"), &index)?;
        cache.index = index;
    }
    Ok(())
}

/// Reads every net file. One that can't be read is renamed out of the way (kept, not
/// deleted) and reported, so the page can offer its newest snapshot.
fn load_from(root: &Path, cache: &mut Cache) -> Result<Loaded, String> {
    *cache = Cache::default();
    let index_path = root.join("index.json");
    let mut ids: Vec<String> = match fs::read_dir(root) {
        Ok(rd) => rd
            .flatten()
            .filter(|e| e.path().is_file())
            .filter_map(|e| e.file_name().into_string().ok())
            .filter_map(|n| n.strip_suffix(".json").map(str::to_string))
            .filter(|n| valid_id(n))
            .collect(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(format!("Couldn't open {}: {}", root.display(), e)),
    };
    if ids.is_empty() && !index_path.is_file() {
        return Ok(Loaded { exists: false, store: String::new(), problems: Vec::new() });
    }
    ids.sort();

    let index: Value = fs::read_to_string(&index_path)
        .ok()
        .and_then(|s| {
            cache.index = s.clone();
            serde_json::from_str(&s).ok()
        })
        .unwrap_or(Value::Null);
    // The saved order first, then any file the index doesn't know about
    let mut ordered: Vec<String> = index["order"]
        .as_array()
        .map(|a| a.iter().filter_map(|x| x.as_str()).filter(|x| ids.iter().any(|i| i == x)).map(str::to_string).collect())
        .unwrap_or_default();
    for id in &ids {
        if !ordered.contains(id) {
            ordered.push(id.clone());
        }
    }

    let mut parts = Vec::new();
    let mut problems = Vec::new();
    let mut good: Vec<&String> = Vec::new();
    for id in &ordered {
        let path = net_path(root, id);
        let parsed = fs::read_to_string(&path).ok().and_then(|s| serde_json::from_str::<Value>(&s).ok()).filter(Value::is_object);
        match parsed {
            Some(net) => {
                let saved = canonical(&net);
                parts.push(format!("{}:{}", Value::String(id.clone()), saved.full));
                cache.nets.insert(id.clone(), saved);
                good.push(id);
            }
            None => {
                let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                let kept_as = format!("{id}.damaged-{secs}.json");
                fs::rename(&path, root.join(&kept_as)).map_err(|e| format!("Couldn't set aside {}: {}", path.display(), e))?;
                problems.push(Problem { id: id.clone(), kept_as, newest_snapshot: snapshot_dates(root, id).into_iter().next() });
            }
        }
    }
    let active = index["activeNetId"]
        .as_str()
        .filter(|a| good.iter().any(|g| g.as_str() == *a))
        .map(str::to_string)
        .or_else(|| good.first().map(|g| g.to_string()))
        .unwrap_or_default();
    let store = format!(r#"{{"version":2,"activeNetId":{},"nets":{{{}}}}}"#, Value::String(active), parts.join(","));
    Ok(Loaded { exists: true, store, problems })
}

fn root(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app.path().app_data_dir().map_err(|e| e.to_string())?.join(DIR_NAME))
}

/// Reads the stored nets at startup.
#[tauri::command]
pub fn store_load(app: AppHandle, state: State<StoreState>) -> Result<Loaded, String> {
    load_from(&root(&app)?, &mut state.0.lock().unwrap())
}

/// Saves the whole store (the page calls this after every change).
#[tauri::command]
pub fn store_save(app: AppHandle, state: State<StoreState>, store: String, today: String) -> Result<(), String> {
    save_to(&root(&app)?, &mut state.0.lock().unwrap(), &store, &today)
}

/// First run with file storage: writes the web view's store to files, reads it back to
/// be sure it arrived whole, and keeps the web view's original text as well.
#[tauri::command]
pub fn store_migrate(
    app: AppHandle,
    state: State<StoreState>,
    store: String,
    original: Option<String>,
    today: String,
) -> Result<(), String> {
    let root = root(&app)?;
    let mut cache = state.0.lock().unwrap();
    migrate_to(&root, &mut cache, &store, original.as_deref(), &today)
}

fn migrate_to(root: &Path, cache: &mut Cache, store: &str, original: Option<&str>, today: &str) -> Result<(), String> {
    if load_from(root, cache)?.exists {
        return Err("Nets are already stored in files here.".into());
    }
    save_to(root, cache, store, today)?;
    let mut check = Cache::default();
    load_from(root, &mut check)?;
    let same = check.nets.len() == cache.nets.len()
        && cache.nets.iter().all(|(id, s)| check.nets.get(id).is_some_and(|c| c.full == s.full));
    if !same {
        // Leave nothing half-moved behind, so the next start tries again from the web view's copy
        for id in cache.nets.keys() {
            let _ = fs::remove_file(net_path(root, id));
        }
        let _ = fs::remove_file(root.join("index.json"));
        *cache = Cache::default();
        return Err("The nets didn't read back the same after being moved to files.".into());
    }
    if let Some(text) = original {
        atomic_write(&root.join(PRE_MIGRATION_FILE), text)?;
    }
    Ok(())
}

/// A net's snapshots, newest first, with "before-restore" (if any) at the end.
#[tauri::command]
pub fn store_snapshots(app: AppHandle, net_id: String) -> Result<Vec<SnapshotInfo>, String> {
    if !valid_id(&net_id) {
        return Err("Bad net name.".into());
    }
    let root = root(&app)?;
    let dir = snapshot_dir(&root, &net_id);
    let modified = |name: &str| {
        fs::metadata(dir.join(format!("{name}.json")))
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
    };
    let mut out: Vec<SnapshotInfo> = snapshot_dates(&root, &net_id)
        .into_iter()
        .map(|name| SnapshotInfo { modified: modified(&name).unwrap_or(0), name })
        .collect();
    if let Some(m) = modified(BEFORE_RESTORE) {
        out.push(SnapshotInfo { name: BEFORE_RESTORE.into(), modified: m });
    }
    Ok(out)
}

/// One snapshot's contents (a whole net as JSON).
#[tauri::command]
pub fn store_snapshot_read(app: AppHandle, net_id: String, name: String) -> Result<String, String> {
    if !valid_id(&net_id) || !(valid_date(&name) || name == BEFORE_RESTORE) {
        return Err("Bad snapshot name.".into());
    }
    let path = snapshot_dir(&root(&app)?, &net_id).join(format!("{name}.json"));
    let text = fs::read_to_string(&path).map_err(|e| format!("Couldn't read that snapshot: {e}"))?;
    match serde_json::from_str::<Value>(&text) {
        Ok(v) if v.is_object() => Ok(text),
        _ => Err("That snapshot is damaged.".into()),
    }
}

/// Keeps the net as it is now, just before a restore replaces it.
#[tauri::command]
pub fn store_keep_before_restore(app: AppHandle, net_id: String, contents: String) -> Result<(), String> {
    if !valid_id(&net_id) {
        return Err("Bad net name.".into());
    }
    if !serde_json::from_str::<Value>(&contents).is_ok_and(|v| v.is_object()) {
        return Err("That isn't a net.".into());
    }
    let dir = snapshot_dir(&root(&app)?, &net_id);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    atomic_write(&dir.join(format!("{BEFORE_RESTORE}.json")), &contents)
}

/// The folder the nets are kept in, for showing to the user.
#[tauri::command]
pub fn store_folder(app: AppHandle) -> Result<String, String> {
    Ok(root(&app)?.to_string_lossy().into_owned())
}

/// Environment for a program started from inside an AppImage: the AppImage points
/// library, Python, GTK and similar settings at its own bundled copies, which make the
/// system's file manager fail to start. Every setting is given back without the parts
/// that point into the AppImage (`None` = remove the variable altogether).
#[cfg(any(target_os = "linux", test))]
fn without_appimage_paths(vars: impl Iterator<Item = (String, String)>, appdir: &str) -> Vec<(String, Option<String>)> {
    let appdir = appdir.trim_end_matches('/');
    if appdir.is_empty() {
        return Vec::new();
    }
    vars.filter(|(k, v)| k != "APPDIR" && v.contains(appdir))
        .map(|(k, v)| {
            let kept: Vec<&str> = v.split(':').filter(|part| !part.is_empty() && !part.starts_with(appdir)).collect();
            (k, (!kept.is_empty()).then(|| kept.join(":")))
        })
        .collect()
}

/// Opens that folder in the system's file manager.
#[tauri::command]
pub fn store_open_folder(app: AppHandle) -> Result<(), String> {
    let dir = root(&app)?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    #[cfg(target_os = "linux")]
    if let Ok(appdir) = std::env::var("APPDIR") {
        let mut cmd = std::process::Command::new("xdg-open");
        cmd.arg(&dir);
        for (key, value) in without_appimage_paths(std::env::vars(), &appdir) {
            match value {
                Some(v) => cmd.env(key, v),
                None => cmd.env_remove(key),
            };
        }
        return cmd.spawn().map(|_| ()).map_err(|e| format!("Couldn't start the file manager: {e}"));
    }
    use tauri_plugin_opener::OpenerExt;
    app.opener().open_path(dir.to_string_lossy(), None::<&str>).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh empty folder under the system temp folder, removed when dropped
    struct TempDir(PathBuf);
    impl TempDir {
        fn new(tag: &str) -> Self {
            let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
            let p = std::env::temp_dir().join(format!("nrl-store-test-{tag}-{}-{nanos}", std::process::id()));
            fs::create_dir_all(&p).unwrap();
            TempDir(p)
        }
        fn root(&self) -> PathBuf {
            self.0.join("nets")
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn store(active: &str, nets: &[(&str, &str, i64)]) -> String {
        let body: Vec<String> = nets
            .iter()
            .map(|(id, name, cursor)| {
                format!(r#""{id}":{{"settings":{{"netName":"{name}"}},"roster":[],"qni":[],"mainCursor":{cursor}}}"#)
            })
            .collect();
        format!(r#"{{"version":2,"activeNetId":"{active}","nets":{{{}}}}}"#, body.join(","))
    }

    #[test]
    fn saves_and_loads_keeping_order() {
        let t = TempDir::new("order");
        let mut c = Cache::default();
        assert!(!load_from(&t.root(), &mut c).unwrap().exists);
        let s = store("net_b", &[("net_z", "Zulu", 0), ("net_b", "Bravo", 0), ("net_a", "Alpha", 0)]);
        save_to(&t.root(), &mut c, &s, "2026-11-16").unwrap();
        let mut fresh = Cache::default();
        let l = load_from(&t.root(), &mut fresh).unwrap();
        assert!(l.exists && l.problems.is_empty());
        let v: Value = serde_json::from_str(&l.store).unwrap();
        assert_eq!(v["activeNetId"], "net_b");
        let order: Vec<&String> = v["nets"].as_object().unwrap().keys().collect();
        assert_eq!(order, ["net_z", "net_b", "net_a"]);
        assert_eq!(v["nets"]["net_a"]["settings"]["netName"], "Alpha");
    }

    #[test]
    fn snapshot_is_the_state_before_the_days_first_change() {
        let t = TempDir::new("snap");
        let mut c = Cache::default();
        let snaps = |t: &TempDir| snapshot_dates(&t.root(), "net_a");
        save_to(&t.root(), &mut c, &store("net_a", &[("net_a", "One", 0)]), "2026-11-10").unwrap();
        assert!(snaps(&t).is_empty(), "a brand-new net has nothing to snapshot");
        // Only the roll-call position moved: saved, but not worth a snapshot
        save_to(&t.root(), &mut c, &store("net_a", &[("net_a", "One", 5)]), "2026-11-11").unwrap();
        assert!(snaps(&t).is_empty());
        save_to(&t.root(), &mut c, &store("net_a", &[("net_a", "Two", 5)]), "2026-11-11").unwrap();
        save_to(&t.root(), &mut c, &store("net_a", &[("net_a", "Three", 5)]), "2026-11-11").unwrap();
        assert_eq!(snaps(&t), ["2026-11-11"]);
        let snap = fs::read_to_string(snapshot_dir(&t.root(), "net_a").join("2026-11-11.json")).unwrap();
        assert!(snap.contains("One"), "the snapshot holds the state before that day's changes");
        let now = fs::read_to_string(net_path(&t.root(), "net_a")).unwrap();
        assert!(now.contains("Three"));
    }

    #[test]
    fn keeps_seven_days_of_snapshots() {
        let t = TempDir::new("seven");
        let mut c = Cache::default();
        for day in 1..=12 {
            let name = format!("Day{day}");
            save_to(&t.root(), &mut c, &store("net_a", &[("net_a", &name, 0)]), &format!("2026-11-{day:02}")).unwrap();
        }
        let dates = snapshot_dates(&t.root(), "net_a");
        assert_eq!(dates.len(), KEEP_SNAPSHOTS);
        assert_eq!(dates.first().unwrap(), "2026-11-12");
        assert_eq!(dates.last().unwrap(), "2026-11-06");
    }

    #[test]
    fn refuses_an_empty_or_malformed_store() {
        let t = TempDir::new("refuse");
        let mut c = Cache::default();
        save_to(&t.root(), &mut c, &store("net_a", &[("net_a", "Keep", 0)]), "2026-11-16").unwrap();
        for bad in [r#"{"version":2,"activeNetId":"x","nets":{}}"#, "{}", "not json", r#"{"nets":{"../evil":{}}}"#, r#"{"nets":{"net_a":5}}"#] {
            assert!(save_to(&t.root(), &mut c, bad, "2026-11-16").is_err(), "{bad}");
        }
        assert!(save_to(&t.root(), &mut c, &store("net_a", &[("net_a", "Keep", 0)]), "16 Nov").is_err());
        assert!(fs::read_to_string(net_path(&t.root(), "net_a")).unwrap().contains("Keep"));
    }

    #[test]
    fn a_deleted_net_is_snapshotted_then_removed() {
        let t = TempDir::new("delete");
        let mut c = Cache::default();
        save_to(&t.root(), &mut c, &store("net_a", &[("net_a", "A", 0), ("net_b", "B", 0)]), "2026-11-15").unwrap();
        save_to(&t.root(), &mut c, &store("net_a", &[("net_a", "A", 0)]), "2026-11-16").unwrap();
        assert!(!net_path(&t.root(), "net_b").exists());
        assert_eq!(snapshot_dates(&t.root(), "net_b"), ["2026-11-16"]);
        assert!(net_path(&t.root(), "net_a").exists());
    }

    #[test]
    fn a_damaged_file_is_set_aside_and_reported() {
        let t = TempDir::new("damaged");
        let mut c = Cache::default();
        save_to(&t.root(), &mut c, &store("net_a", &[("net_a", "A", 0), ("net_b", "B1", 0)]), "2026-11-15").unwrap();
        save_to(&t.root(), &mut c, &store("net_a", &[("net_a", "A", 0), ("net_b", "B2", 0)]), "2026-11-16").unwrap();
        fs::write(net_path(&t.root(), "net_b"), "{ half a fi").unwrap();
        let mut fresh = Cache::default();
        let l = load_from(&t.root(), &mut fresh).unwrap();
        assert_eq!(l.problems.len(), 1);
        let p = &l.problems[0];
        assert_eq!(p.id, "net_b");
        assert_eq!(p.newest_snapshot.as_deref(), Some("2026-11-16"));
        assert!(t.root().join(&p.kept_as).is_file(), "the damaged file is kept");
        let v: Value = serde_json::from_str(&l.store).unwrap();
        assert!(v["nets"]["net_a"].is_object() && v["nets"]["net_b"].is_null());
        // A later save of the remaining net must not disturb the set-aside file
        save_to(&t.root(), &mut fresh, &store("net_a", &[("net_a", "A", 1)]), "2026-11-16").unwrap();
        assert!(t.root().join(&p.kept_as).is_file());
    }

    #[test]
    fn migration_writes_checks_and_keeps_the_original() {
        let t = TempDir::new("migrate");
        let mut c = Cache::default();
        let s = store("net_a", &[("net_a", "Alpha", 3), ("net_b", "Bravo", 0)]);
        migrate_to(&t.root(), &mut c, &s, Some("the original text"), "2026-11-16").unwrap();
        assert_eq!(fs::read_to_string(t.root().join(PRE_MIGRATION_FILE)).unwrap(), "the original text");
        let mut fresh = Cache::default();
        let l = load_from(&t.root(), &mut fresh).unwrap();
        assert!(l.exists && l.problems.is_empty(), "the kept original isn't mistaken for a net");
        assert_eq!(serde_json::from_str::<Value>(&l.store).unwrap()["nets"].as_object().unwrap().len(), 2);
        assert!(migrate_to(&t.root(), &mut c, &s, None, "2026-11-16").is_err(), "never migrates over existing files");
    }

    /// Rehearses the move to files on a real store, in a temporary folder:
    /// `NRL_REHEARSAL=/path/to/store.json cargo test --lib -- --ignored rehearse --nocapture`
    #[test]
    #[ignore]
    fn rehearse_migration_on_a_real_store() {
        let path = std::env::var("NRL_REHEARSAL").expect("set NRL_REHEARSAL to a store JSON file");
        let text = fs::read_to_string(path).unwrap();
        let before: Value = serde_json::from_str(&text).unwrap();
        let t = TempDir::new("rehearse");
        let mut c = Cache::default();
        migrate_to(&t.root(), &mut c, &text, Some(&text), "2026-11-16").unwrap();
        let mut fresh = Cache::default();
        let l = load_from(&t.root(), &mut fresh).unwrap();
        assert!(l.exists && l.problems.is_empty());
        let after: Value = serde_json::from_str(&l.store).unwrap();
        assert_eq!(after["nets"], before["nets"], "every net reads back exactly as it was");
        assert_eq!(after["activeNetId"], before["activeNetId"]);
        let order = |v: &Value| v["nets"].as_object().unwrap().keys().cloned().collect::<Vec<_>>();
        assert_eq!(order(&after), order(&before), "nets stay in the same order");
        assert_eq!(fs::read_to_string(t.root().join(PRE_MIGRATION_FILE)).unwrap(), text);
        println!("{} net(s) moved and read back identical, {} bytes", order(&after).len(), text.len());
    }

    #[test]
    fn strips_appimage_paths_from_the_environment() {
        let appdir = "/tmp/.mount_N8VCL abc";
        let vars = [
            ("PATH", "/tmp/.mount_N8VCL abc/usr/bin/:/tmp/.mount_N8VCL abc/bin/:/usr/local/bin:/usr/bin"),
            ("PYTHONHOME", "/tmp/.mount_N8VCL abc/usr/"),
            ("PYTHONPATH", "/tmp/.mount_N8VCL abc/usr/share/pyshared/:"),
            ("XDG_DATA_DIRS", "/tmp/.mount_N8VCL abc/usr/share/:/usr/share:/usr/local/share"),
            ("HOME", "/home/someone"),
            ("APPDIR", "/tmp/.mount_N8VCL abc"),
        ];
        let out = without_appimage_paths(vars.iter().map(|(k, v)| (k.to_string(), v.to_string())), appdir);
        let get = |k: &str| out.iter().find(|(key, _)| key == k).map(|(_, v)| v.clone());
        assert_eq!(get("PATH"), Some(Some("/usr/local/bin:/usr/bin".into())));
        assert_eq!(get("PYTHONHOME"), Some(None));
        assert_eq!(get("PYTHONPATH"), Some(None));
        assert_eq!(get("XDG_DATA_DIRS"), Some(Some("/usr/share:/usr/local/share".into())));
        assert_eq!(get("HOME"), None, "untouched variables aren't listed");
        assert_eq!(get("APPDIR"), None);
    }

    #[test]
    fn checks_names() {
        assert!(valid_id("net_mb3x9k2a1b") && valid_id("a-b_C9"));
        for bad in ["", "index", "../x", "a.b", "a/b", "a b"] {
            assert!(!valid_id(bad), "{bad}");
        }
        assert!(valid_date("2026-11-16"));
        for bad in ["2026-11-1", "2026/11/16", "before-restore", "20261116xx"] {
            assert!(!valid_date(bad), "{bad}");
        }
    }
}
