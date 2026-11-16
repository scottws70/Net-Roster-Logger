//! Outgoing email (SMTP): stores the user's mail server login on this device and sends
//! messages with a file attached, such as the Net Report CSV for the Net Manager.
//! The password is kept in the app's config folder (readable only by this user) and
//! is never sent back to the web page.

use lettre::message::header::ContentType;
use lettre::message::{Attachment, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize, Default, Clone)]
struct MailSettings {
    host: String,
    port: u16,
    /// "starttls" (usually port 587) or "ssl" (usually port 465)
    security: String,
    username: String,
    /// The From address; the username when blank
    from: String,
    password: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MailStatus {
    host: String,
    port: u16,
    security: String,
    username: String,
    from: String,
    has_password: bool,
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("mail.json"))
}

fn load(app: &AppHandle) -> MailSettings {
    settings_path(app)
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

fn status_of(s: &MailSettings) -> MailStatus {
    MailStatus {
        host: s.host.clone(),
        port: s.port,
        security: s.security.clone(),
        username: s.username.clone(),
        from: s.from.clone(),
        has_password: !s.password.is_empty(),
    }
}

fn from_address(s: &MailSettings) -> &str {
    if s.from.trim().is_empty() { s.username.trim() } else { s.from.trim() }
}

fn transport(s: &MailSettings) -> Result<SmtpTransport, String> {
    if s.host.trim().is_empty() || s.username.trim().is_empty() || s.password.is_empty() {
        return Err("Set up email in Settings first: mail server, username, and password.".into());
    }
    let host = s.host.trim();
    let builder = if s.security == "ssl" {
        SmtpTransport::relay(host)
    } else {
        SmtpTransport::starttls_relay(host)
    }
    .map_err(|e| format!("Mail server problem: {e}"))?;
    let port = if s.port == 0 { if s.security == "ssl" { 465 } else { 587 } } else { s.port };
    Ok(builder
        .port(port)
        .credentials(Credentials::new(s.username.trim().to_string(), s.password.clone()))
        .timeout(Some(Duration::from_secs(30)))
        .build())
}

/// Turns lettre's errors into something a user can act on.
fn friendly(e: lettre::transport::smtp::Error) -> String {
    let text = e.to_string();
    let lower = text.to_lowercase();
    if lower.contains("535") || lower.contains("534") || lower.contains("authentication") || lower.contains("credentials") {
        format!("The mail server didn't accept the username or password. Gmail and Yahoo need an app password, not your normal one. ({text})")
    } else if lower.contains("timed out") || lower.contains("timeout") {
        format!("Couldn't reach the mail server. Check the server name, port, and internet connection. ({text})")
    } else {
        text
    }
}

#[tauri::command]
pub fn mail_status(app: AppHandle) -> MailStatus {
    status_of(&load(&app))
}

/// Saves the settings, and the password if one is given (a blank password keeps the saved one).
#[tauri::command]
pub fn mail_save_settings(
    app: AppHandle,
    host: String,
    port: u16,
    security: String,
    username: String,
    from: String,
    password: Option<String>,
) -> Result<MailStatus, String> {
    let mut s = load(&app);
    s.host = host.trim().to_string();
    s.port = port;
    s.security = if security == "ssl" { "ssl".into() } else { "starttls".into() };
    s.username = username.trim().to_string();
    s.from = from.trim().to_string();
    if let Some(p) = password.filter(|p| !p.is_empty()) {
        s.password = p;
    }
    let json = serde_json::to_string(&s).map_err(|e| e.to_string())?;
    write_private(&settings_path(&app)?, &json).map_err(|e| e.to_string())?;
    Ok(status_of(&s))
}

#[tauri::command]
pub fn mail_clear_settings(app: AppHandle) -> Result<(), String> {
    let p = settings_path(&app)?;
    if p.exists() {
        fs::remove_file(p).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Connects and signs in to the mail server without sending anything.
#[tauri::command]
pub async fn mail_test(app: AppHandle) -> Result<String, String> {
    let s = load(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let t = transport(&s)?;
        match t.test_connection() {
            Ok(true) => Ok(format!("Signed in to {} as {}.", s.host, s.username)),
            Ok(false) => Err("The mail server didn't respond.".into()),
            Err(e) => Err(friendly(e)),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Sends a plain-text email with one text file (e.g. a CSV) attached. Blocking; also used
/// by Sharing so a paired device (the tablet) can send through this computer's account.
pub(crate) fn send_blocking(
    app: &AppHandle,
    to: &str,
    subject: &str,
    body: &str,
    file_name: &str,
    file_contents: &str,
    content_type: Option<&str>,
) -> Result<String, String> {
    let s = load(app);
    let from: Mailbox = from_address(&s)
        .parse()
        .map_err(|_| format!("\"{}\" isn't a valid From address. Check Settings > Email.", from_address(&s)))?;
    let to = to.trim();
    let to_box: Mailbox = to
        .parse()
        .map_err(|_| format!("\"{to}\" isn't a valid email address."))?;
    let ctype = ContentType::parse(content_type.unwrap_or("text/csv")).unwrap_or(ContentType::TEXT_PLAIN);
    let msg = Message::builder()
        .from(from)
        .to(to_box)
        .subject(subject)
        .multipart(
            MultiPart::mixed()
                .singlepart(SinglePart::plain(body.to_string()))
                .singlepart(Attachment::new(file_name.to_string()).body(file_contents.as_bytes().to_vec(), ctype)),
        )
        .map_err(|e| e.to_string())?;
    let t = transport(&s)?;
    t.send(&msg).map_err(friendly)?;
    Ok(format!("Sent to {to}."))
}

#[tauri::command]
pub async fn mail_send(
    app: AppHandle,
    to: String,
    subject: String,
    body: String,
    file_name: String,
    file_contents: String,
    content_type: Option<String>,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        send_blocking(&app, &to, &subject, &body, &file_name, &file_contents, content_type.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
}
