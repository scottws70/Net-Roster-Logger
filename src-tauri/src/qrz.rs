//! QRZ.com XML lookup service: stores the user's QRZ login on this computer, signs
//! in to get a session key, and downloads station photos into the app's data folder.
//! The password is kept in the app's config folder (readable only by this user) and
//! is never sent back to the web page.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

const QRZ_URL: &str = "https://xmldata.qrz.com/xml/current/";
const AGENT: &str = concat!("N8VCLNetLogger", env!("CARGO_PKG_VERSION"));

/// The session key from the last successful login, reused until QRZ rejects it.
#[derive(Default)]
pub struct QrzSession(pub Mutex<Option<String>>);

#[derive(Serialize, Deserialize, Default)]
struct Credentials {
    username: String,
    password: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QrzStatus {
    username: String,
    has_password: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResult {
    ok: bool,
    /// Subscription expiry date, or "non-subscriber"
    sub_exp: Option<String>,
    message: Option<String>,
    error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LookupResult {
    /// "saved", "noimage" (on QRZ but no photo), or "notfound" (callsign not on QRZ)
    status: String,
    /// Photo file name inside the photos folder, when saved
    file: Option<String>,
    /// QRZ's first name (may include a middle initial, e.g. "FRED L"), last name, and on-air nickname
    first_name: Option<String>,
    last_name: Option<String>,
    nickname: Option<String>,
    /// Mailing address: street (addr1), city (addr2), state (USA only), ZIP
    street: Option<String>,
    city: Option<String>,
    state: Option<String>,
    zip: Option<String>,
}

fn credentials_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("qrz.json"))
}

fn load_credentials(app: &AppHandle) -> Credentials {
    credentials_path(app)
        .ok()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_private(path: &PathBuf, contents: &str) -> std::io::Result<()> {
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    use std::io::Write;
    opts.open(path)?.write_all(contents.as_bytes())
}

fn status_of(c: &Credentials) -> QrzStatus {
    QrzStatus {
        username: c.username.clone(),
        has_password: !c.password.is_empty(),
    }
}

/// Text of the first `<name>...</name>` element, with the basic XML escapes undone.
fn xml_tag(xml: &str, name: &str) -> Option<String> {
    let open = format!("<{}>", name);
    let close = format!("</{}>", name);
    let start = xml.find(&open)? + open.len();
    let len = xml[start..].find(&close)?;
    let text = xml[start..start + len]
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&");
    let text = text.trim().to_string();
    (!text.is_empty()).then_some(text)
}

fn photos_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("photos");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// One request to the XML service (blocking).
fn qrz_get(params: &[(&str, &str)]) -> Result<String, String> {
    let mut req = ureq::get(QRZ_URL).query("agent", AGENT);
    for (k, v) in params {
        req = req.query(k, v);
    }
    req.call()
        .map_err(|e| format!("Couldn't reach QRZ: {}", e))?
        .into_string()
        .map_err(|e| e.to_string())
}

fn login_xml(c: &Credentials) -> Result<String, String> {
    if c.username.is_empty() || c.password.is_empty() {
        return Err("Enter and save your QRZ username and password first.".into());
    }
    qrz_get(&[("username", &c.username), ("password", &c.password)])
}

/// Looks up a callsign, reusing the cached session key and signing in again
/// whenever QRZ reports the session is gone (its reply then has no Key).
fn lookup_xml(app: &AppHandle, callsign: &str) -> Result<String, String> {
    let session = app.state::<QrzSession>();
    let cached = session.0.lock().unwrap().clone();
    if let Some(key) = cached {
        let xml = qrz_get(&[("s", &key), ("callsign", callsign)])?;
        if xml_tag(&xml, "Key").is_some() {
            return Ok(xml);
        }
    }
    let login = login_xml(&load_credentials(app))?;
    let key = xml_tag(&login, "Key").ok_or_else(|| {
        format!("QRZ login failed: {}", xml_tag(&login, "Error").unwrap_or_else(|| "no session returned".into()))
    })?;
    *session.0.lock().unwrap() = Some(key.clone());
    qrz_get(&[("s", &key), ("callsign", callsign)])
}

/// Photo file name for a callsign: "W1AW.jpg", "W1AW_M.png" for W1AW/M.
fn photo_file_name(callsign: &str, ext: &str) -> String {
    let safe: String = callsign
        .to_uppercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    format!("{}.{}", safe, ext)
}

fn image_ext(url: &str, content_type: &str) -> &'static str {
    let path = url.split(['?', '#']).next().unwrap_or("").to_ascii_lowercase();
    for (ext, name) in [(".png", "png"), (".gif", "gif"), (".webp", "webp"), (".jpg", "jpg"), (".jpeg", "jpg")] {
        if path.ends_with(ext) {
            return name;
        }
    }
    match content_type {
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        _ => "jpg",
    }
}

fn lookup_blocking(app: &AppHandle, callsign: &str) -> Result<LookupResult, String> {
    let xml = lookup_xml(app, callsign)?;
    let mut result = LookupResult {
        status: "notfound".into(),
        file: None,
        first_name: xml_tag(&xml, "fname"),
        last_name: xml_tag(&xml, "name"),
        nickname: xml_tag(&xml, "nickname"),
        street: xml_tag(&xml, "addr1"),
        city: xml_tag(&xml, "addr2"),
        state: xml_tag(&xml, "state"),
        zip: xml_tag(&xml, "zip"),
    };
    if xml_tag(&xml, "Callsign").is_none() {
        let err = xml_tag(&xml, "Error").unwrap_or_default();
        if err.to_ascii_lowercase().starts_with("not found") {
            return Ok(result);
        }
        return Err(if err.is_empty() { "QRZ returned no data for that callsign.".into() } else { err });
    }
    let Some(url) = xml_tag(&xml, "image") else {
        result.status = "noimage".into();
        return Ok(result);
    };

    let resp = ureq::get(&url).call().map_err(|e| format!("Couldn't download the photo: {}", e))?;
    let ext = image_ext(&url, resp.content_type());
    let mut bytes = Vec::new();
    // QRZ photos are normally well under this; the cap guards against a runaway download
    resp.into_reader()
        .take(15 * 1024 * 1024)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;

    let dir = photos_dir(app)?;
    // Drop any earlier photo for this callsign saved under another extension
    for old in ["jpg", "png", "gif", "webp"] {
        let _ = fs::remove_file(dir.join(photo_file_name(callsign, old)));
    }
    let file = photo_file_name(callsign, ext);
    fs::write(dir.join(&file), bytes).map_err(|e| e.to_string())?;
    result.status = "saved".into();
    result.file = Some(file);
    Ok(result)
}

#[tauri::command]
pub fn qrz_photos_dir(app: AppHandle) -> Result<String, String> {
    Ok(photos_dir(&app)?.to_string_lossy().into_owned())
}

/// Names of the photo files saved on this computer, so the page can spot stations
/// whose photo is missing here (e.g. after restoring a backup made elsewhere).
#[tauri::command]
pub fn qrz_saved_photos(app: AppHandle) -> Result<Vec<String>, String> {
    let entries = fs::read_dir(photos_dir(&app)?).map_err(|e| e.to_string())?;
    Ok(entries
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect())
}

/// Looks a station up on QRZ, saves its primary photo if it has one, and returns its names.
#[tauri::command]
pub async fn qrz_lookup(app: AppHandle, callsign: String) -> Result<LookupResult, String> {
    let callsign = callsign.trim().to_uppercase();
    if callsign.is_empty() {
        return Err("No callsign given.".into());
    }
    tauri::async_runtime::spawn_blocking(move || lookup_blocking(&app, &callsign))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn qrz_status(app: AppHandle) -> QrzStatus {
    status_of(&load_credentials(&app))
}

/// Saves the username, and the password if one is given (a blank password keeps the saved one).
#[tauri::command]
pub fn qrz_save_credentials(
    app: AppHandle,
    session: State<QrzSession>,
    username: String,
    password: Option<String>,
) -> Result<QrzStatus, String> {
    let mut c = load_credentials(&app);
    c.username = username.trim().to_string();
    if let Some(p) = password.filter(|p| !p.is_empty()) {
        c.password = p;
    }
    let json = serde_json::to_string(&c).map_err(|e| e.to_string())?;
    write_private(&credentials_path(&app)?, &json).map_err(|e| e.to_string())?;
    *session.0.lock().unwrap() = None;
    Ok(status_of(&c))
}

#[tauri::command]
pub fn qrz_clear_credentials(app: AppHandle, session: State<QrzSession>) -> Result<(), String> {
    let path = credentials_path(&app)?;
    if path.exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    *session.0.lock().unwrap() = None;
    Ok(())
}

/// Signs in to QRZ with the saved login and reports the subscription status.
#[tauri::command]
pub async fn qrz_test_login(app: AppHandle) -> Result<LoginResult, String> {
    let c = load_credentials(&app);
    // ureq is blocking, so keep it off the UI thread
    let xml = tauri::async_runtime::spawn_blocking(move || login_xml(&c))
        .await
        .map_err(|e| e.to_string())??;

    let key = xml_tag(&xml, "Key");
    *app.state::<QrzSession>().0.lock().unwrap() = key.clone();
    Ok(LoginResult {
        ok: key.is_some(),
        sub_exp: xml_tag(&xml, "SubExp"),
        message: xml_tag(&xml, "Message"),
        error: xml_tag(&xml, "Error"),
    })
}

#[cfg(test)]
mod tests {
    use super::{image_ext, photo_file_name, xml_tag};

    #[test]
    fn names_photo_files() {
        assert_eq!(photo_file_name("w1aw", "jpg"), "W1AW.jpg");
        assert_eq!(photo_file_name("W1AW/M", "png"), "W1AW_M.png");
        assert_eq!(image_ext("https://files.qrz.com/q/aa7bq/aa7bq.JPG", ""), "jpg");
        assert_eq!(image_ext("https://files.qrz.com/x/pic.png?x=1", "image/jpeg"), "png");
        assert_eq!(image_ext("https://files.qrz.com/x/pic", "image/gif"), "gif");
    }

    #[test]
    fn reads_session_fields() {
        let ok = "<?xml version=\"1.0\" ?><QRZDatabase version=\"1.34\"><Session>\
                  <Key>2331uf894c4bd29f3923f3bacf02c532d7bd9</Key><Count>123</Count>\
                  <SubExp>Wed Jan 1 12:34:03 2013</SubExp></Session></QRZDatabase>";
        assert_eq!(xml_tag(ok, "Key").as_deref(), Some("2331uf894c4bd29f3923f3bacf02c532d7bd9"));
        assert_eq!(xml_tag(ok, "SubExp").as_deref(), Some("Wed Jan 1 12:34:03 2013"));
        assert_eq!(xml_tag(ok, "Error"), None);

        let bad = "<QRZDatabase><Session><Error>Username/password incorrect &amp; locked</Error></Session></QRZDatabase>";
        assert_eq!(xml_tag(bad, "Key"), None);
        assert_eq!(xml_tag(bad, "Error").as_deref(), Some("Username/password incorrect & locked"));
    }
}
