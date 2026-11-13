//! Sharing this computer's nets with other devices on the LAN or Tailscale.
//!
//! When sharing is on, a small web server runs on this computer. It serves the app page
//! itself, so a tablet, phone, or laptop can open `http://<this computer>:<port>` in a
//! browser (or "Connect" from the installed app) and work on this computer's data.
//!
//! The page here stays the owner of the data (its localStorage). It hands every saved
//! copy to `share_publish`, and a remote device's saves are passed back to it as a
//! `share-store` event. Each change bumps a version number that remote pages poll, so
//! whichever device is in use, the others follow along within a second or two.
//!
//! Everything except the page itself and `/api/hello` needs the pairing code, sent as a
//! cookie (set by `/api/pair`) or an `X-Pairing-Code` header. Wrong codes are throttled.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::fs;
use std::hash::{BuildHasher, Hasher};
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};
use tiny_http::{Header, Method, Request, Response, Server};

use crate::{qrz, weather};

const DEFAULT_PORT: u16 = 7373;
const PAGE: &str = include_str!("../../dist/index.html");
const MAX_BODY: u64 = 64 * 1024 * 1024;
const COOKIE: &str = "nl_code";

#[derive(Serialize, Deserialize, Clone)]
struct ShareConfig {
    enabled: bool,
    port: u16,
    code: String,
}

impl Default for ShareConfig {
    fn default() -> Self {
        ShareConfig { enabled: false, port: DEFAULT_PORT, code: new_code() }
    }
}

#[derive(Default)]
struct StoreCopy {
    version: u64,
    /// The whole store as JSON, exactly as the page saved it ("" until first published)
    json: String,
}

#[derive(Default)]
pub struct ShareState {
    inner: Arc<Inner>,
}

#[derive(Default)]
struct Inner {
    store: Mutex<StoreCopy>,
    server: Mutex<Option<Arc<Server>>>,
    error: Mutex<Option<String>>,
    /// Times of recent wrong pairing codes, for throttling guesses
    fails: Mutex<VecDeque<Instant>>,
    code: Mutex<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareStatus {
    enabled: bool,
    running: bool,
    port: u16,
    code: String,
    /// This computer's IPv4 addresses (LAN and Tailscale), for the address to type
    addresses: Vec<String>,
    error: Option<String>,
}

/// A random 6-digit pairing code.
fn new_code() -> String {
    let n = std::collections::hash_map::RandomState::new().build_hasher().finish();
    format!("{:06}", n % 1_000_000)
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("share.json"))
}

fn load_config(app: &AppHandle) -> ShareConfig {
    config_path(app)
        .ok()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_config(app: &AppHandle, cfg: &ShareConfig) -> Result<(), String> {
    let path = config_path(app)?;
    fs::write(&path, serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

fn local_addresses() -> Vec<String> {
    let mut v: Vec<String> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|i| !i.is_loopback())
        // Skip virtual networks no other device can reach (Docker, VMs)
        .filter(|i| !["docker", "br-", "veth", "virbr", "vmnet", "vboxnet"].iter().any(|p| i.name.starts_with(p)))
        .filter_map(|i| match i.ip() {
            std::net::IpAddr::V4(ip) if !ip.is_link_local() => Some(ip.to_string()),
            _ => None,
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

fn status(app: &AppHandle, inner: &Inner) -> ShareStatus {
    let cfg = load_config(app);
    ShareStatus {
        enabled: cfg.enabled,
        running: inner.server.lock().unwrap().is_some(),
        port: cfg.port,
        code: cfg.code,
        addresses: local_addresses(),
        error: inner.error.lock().unwrap().clone(),
    }
}

fn stop(inner: &Inner) {
    if let Some(server) = inner.server.lock().unwrap().take() {
        server.unblock();
    }
}

fn start(app: &AppHandle, inner: &Arc<Inner>, cfg: &ShareConfig) {
    stop(inner);
    *inner.code.lock().unwrap() = cfg.code.clone();
    match Server::http(("0.0.0.0", cfg.port)) {
        Ok(server) => {
            *inner.error.lock().unwrap() = None;
            let server = Arc::new(server);
            *inner.server.lock().unwrap() = Some(server.clone());
            let app = app.clone();
            let inner = inner.clone();
            std::thread::spawn(move || {
                for req in server.incoming_requests() {
                    let app = app.clone();
                    let inner = inner.clone();
                    std::thread::spawn(move || handle(&app, &inner, req));
                }
            });
        }
        Err(e) => {
            *inner.error.lock().unwrap() =
                Some(format!("Couldn't start sharing on port {}: {}", cfg.port, e));
        }
    }
}

/// Starts sharing at launch if it was left on.
pub fn init(app: &AppHandle) {
    let cfg = load_config(app);
    if cfg.enabled {
        let state = app.state::<ShareState>();
        start(app, &state.inner, &cfg);
    }
}

#[tauri::command]
pub fn share_status(app: AppHandle, state: State<ShareState>) -> ShareStatus {
    status(&app, &state.inner)
}

/// Turns sharing on or off (and changes the port when given).
#[tauri::command]
pub fn share_set(app: AppHandle, state: State<ShareState>, enabled: bool, port: Option<u16>) -> Result<ShareStatus, String> {
    let mut cfg = load_config(&app);
    cfg.enabled = enabled;
    if let Some(p) = port {
        if p < 1024 {
            return Err("Use a port number from 1024 to 65535.".into());
        }
        cfg.port = p;
    }
    save_config(&app, &cfg)?;
    if enabled { start(&app, &state.inner, &cfg) } else { stop(&state.inner); *state.inner.error.lock().unwrap() = None; }
    Ok(status(&app, &state.inner))
}

/// Makes a new pairing code; devices paired with the old one must enter the new one.
#[tauri::command]
pub fn share_new_code(app: AppHandle, state: State<ShareState>) -> Result<ShareStatus, String> {
    let mut cfg = load_config(&app);
    cfg.code = new_code();
    save_config(&app, &cfg)?;
    *state.inner.code.lock().unwrap() = cfg.code.clone();
    Ok(status(&app, &state.inner))
}

/// The page here hands over each saved copy of the store. Returns the new version.
#[tauri::command]
pub fn share_publish(state: State<ShareState>, store: String) -> u64 {
    let mut s = state.inner.store.lock().unwrap();
    s.version += 1;
    s.json = store;
    s.version
}

/// Checks that `base` (e.g. "http://192.168.1.20:7373") is a Net Logger that's sharing.
#[tauri::command]
pub async fn share_probe(base: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let url = format!("{}/api/hello", base.trim_end_matches('/'));
        let resp = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(6))
            .build()
            .get(&url)
            .call()
            .map_err(|e| match e {
                ureq::Error::Transport(t) => format!("No answer from {} ({}).", base, t.kind()),
                ureq::Error::Status(code, _) => format!("{} answered with error {}.", base, code),
            })?;
        let v: Value = serde_json::from_str(&resp.into_string().map_err(|e| e.to_string())?)
            .map_err(|_| format!("{} isn't a Net Logger.", base))?;
        if v["app"] == "N8VCL Net Logger" { Ok(()) } else { Err(format!("{} isn't a Net Logger.", base)) }
    })
    .await
    .map_err(|e| e.to_string())?
}

/* ------------------------------ HTTP ------------------------------ */

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).unwrap()
}

fn json_response(code: u16, body: String) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_string(body)
        .with_status_code(code)
        .with_header(header("Content-Type", "application/json; charset=utf-8"))
        .with_header(header("Cache-Control", "no-store"))
}

fn error(code: u16, msg: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    json_response(code, json!({ "error": msg }).to_string())
}

fn req_header<'a>(req: &'a Request, name: &str) -> Option<&'a str> {
    req.headers()
        .iter()
        .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str())
}

/// The pairing code the request carries, from its header or cookie.
fn presented_code(req: &Request) -> Option<String> {
    if let Some(c) = req_header(req, "X-Pairing-Code") {
        return Some(c.trim().to_string());
    }
    req_header(req, "Cookie").and_then(|c| {
        c.split(';')
            .filter_map(|kv| kv.trim().split_once('='))
            .find(|(k, _)| *k == COOKIE)
            .map(|(_, v)| v.trim().to_string())
    })
}

/// Too many wrong codes lately? (5 in 10 seconds locks guessing out for a while.)
fn throttled(inner: &Inner) -> bool {
    let mut f = inner.fails.lock().unwrap();
    while f.front().is_some_and(|t| t.elapsed() > Duration::from_secs(10)) {
        f.pop_front();
    }
    f.len() >= 5
}

fn note_fail(inner: &Inner) {
    inner.fails.lock().unwrap().push_back(Instant::now());
}

fn code_ok(inner: &Inner, given: &str) -> bool {
    let code = inner.code.lock().unwrap();
    !code.is_empty() && given == code.as_str()
}

/// A request that changes something must come from a page on this server, not from
/// some other website open in the same browser.
fn same_origin(req: &Request) -> bool {
    match (req_header(req, "Origin"), req_header(req, "Host")) {
        (None, _) => true,
        (Some(origin), Some(host)) => origin.split_once("://").map(|(_, h)| h) == Some(host),
        _ => false,
    }
}

fn read_body(req: &mut Request) -> Result<String, String> {
    let mut s = String::new();
    req.as_reader().take(MAX_BODY).read_to_string(&mut s).map_err(|e| e.to_string())?;
    Ok(s)
}

/// The app page, with what it needs to run as a remote: whether this browser is paired,
/// and if so the current data, so it opens straight onto it.
fn page(inner: &Inner, paired: bool) -> String {
    let info = if paired {
        let s = inner.store.lock().unwrap();
        if s.json.is_empty() {
            json!({ "paired": true, "ready": false }).to_string()
        } else {
            format!(r#"{{"paired":true,"ready":true,"version":{},"store":{}}}"#, s.version, s.json)
        }
    } else {
        json!({ "paired": false }).to_string()
    };
    // Keep user text in the data from closing the <script> tag early
    let info = info.replace("</", "<\\/").replace("<!--", "<\\!--");
    let inject = format!("<script>window.HAMNET_REMOTE = {};</script>\n<script>", info);
    PAGE.replacen("<script>", &inject, 1)
}

fn handle(app: &AppHandle, inner: &Arc<Inner>, mut req: Request) {
    let url = req.url().to_string();
    let (path, _query) = url.split_once('?').unwrap_or((&url, ""));
    let path = path.to_string();
    let method = req.method().clone();

    let given = presented_code(&req);
    let paired = match &given {
        Some(c) if !throttled(inner) => {
            let ok = code_ok(inner, c);
            if !ok { note_fail(inner); }
            ok
        }
        _ => false,
    };

    let resp = match (method, path.as_str()) {
        (Method::Get, "/") | (Method::Get, "/index.html") => Response::from_string(page(inner, paired))
            .with_header(header("Content-Type", "text/html; charset=utf-8"))
            .with_header(header("Cache-Control", "no-store")),

        (Method::Get, "/api/hello") => json_response(
            200,
            json!({ "app": "N8VCL Net Logger", "version": env!("CARGO_PKG_VERSION") }).to_string(),
        ),

        (Method::Post, "/api/pair") => {
            if !same_origin(&req) {
                error(403, "Not allowed.")
            } else if throttled(inner) {
                error(429, "Too many wrong codes. Wait a few seconds and try again.")
            } else {
                let body = read_body(&mut req).unwrap_or_default();
                let code = serde_json::from_str::<Value>(&body)
                    .ok()
                    .and_then(|v| v["code"].as_str().map(|s| s.trim().to_string()))
                    .unwrap_or_default();
                if code_ok(inner, &code) {
                    json_response(200, r#"{"ok":true}"#.into()).with_header(header(
                        "Set-Cookie",
                        &format!("{}={}; Path=/; Max-Age=31536000; HttpOnly; SameSite=Strict", COOKIE, code),
                    ))
                } else {
                    note_fail(inner);
                    error(403, "That code doesn't match. Check the code in Settings → Sharing on the main computer.")
                }
            }
        }

        (Method::Post, "/api/unpair") => json_response(200, r#"{"ok":true}"#.into())
            .with_header(header("Set-Cookie", &format!("{}=; Path=/; Max-Age=0; HttpOnly; SameSite=Strict", COOKIE))),

        (_, p) if p.starts_with("/api/") || p.starts_with("/photos/") => {
            if !paired {
                error(401, "Not paired with this computer.")
            } else {
                api(app, inner, &mut req, p)
            }
        }

        _ => error(404, "Not found."),
    };
    let _ = req.respond(resp);
}

fn api(app: &AppHandle, inner: &Arc<Inner>, req: &mut Request, path: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    let method = req.method().clone();
    match (method, path) {
        (Method::Get, "/api/version") => {
            let v = inner.store.lock().unwrap().version;
            json_response(200, json!({ "version": v }).to_string())
        }
        (Method::Get, "/api/store") => {
            let s = inner.store.lock().unwrap();
            if s.json.is_empty() {
                error(503, "The main computer is still starting up.")
            } else {
                json_response(200, format!(r#"{{"version":{},"store":{}}}"#, s.version, s.json))
            }
        }
        (Method::Put, "/api/store") => {
            if !same_origin(req) {
                return error(403, "Not allowed.");
            }
            let body = match read_body(req) {
                Ok(b) => b,
                Err(e) => return error(400, &e),
            };
            // Only accept something shaped like the app's data
            let ok = serde_json::from_str::<Value>(&body)
                .map(|v| v["nets"].is_object() && v["activeNetId"].is_string())
                .unwrap_or(false);
            if !ok {
                return error(400, "That isn't Net Logger data.");
            }
            let version = {
                let mut s = inner.store.lock().unwrap();
                s.version += 1;
                s.json = body.clone();
                s.version
            };
            // Hand it to the page on this computer, which saves it and redraws
            let _ = app.emit("share-store", json!({ "version": version, "store": body }));
            json_response(200, json!({ "version": version }).to_string())
        }
        (Method::Post, "/api/invoke") => {
            if !same_origin(req) {
                return error(403, "Not allowed.");
            }
            let body = read_body(req).unwrap_or_default();
            let v: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
            let args = &v["args"];
            let result: Result<Value, String> = match v["cmd"].as_str().unwrap_or("") {
                "qrz_status" => serde_json::to_value(qrz::qrz_status(app.clone())).map_err(|e| e.to_string()),
                "qrz_saved_photos" => qrz::qrz_saved_photos(app.clone()).map(|v| json!(v)),
                "qrz_lookup" => {
                    let call = args["callsign"].as_str().unwrap_or("").trim().to_uppercase();
                    let refresh = args["refreshPhoto"].as_bool().unwrap_or(true);
                    if call.is_empty() {
                        Err("No callsign given.".into())
                    } else {
                        qrz::lookup_blocking(app, &call, refresh)
                            .and_then(|r| serde_json::to_value(r).map_err(|e| e.to_string()))
                    }
                }
                "weather_has_wu_key" => Ok(json!(weather::weather_has_wu_key(app.clone()))),
                "weather_fetch" => {
                    let key = weather::load_keys(app).wu_api_key;
                    weather::fetch_blocking(
                        &key,
                        args["source"].as_str().unwrap_or(""),
                        args["station"].as_str().unwrap_or(""),
                    )
                    .and_then(|w| serde_json::to_value(w).map_err(|e| e.to_string()))
                }
                _ => Err("Not available from another device.".into()),
            };
            match result {
                Ok(v) => json_response(200, json!({ "ok": v }).to_string()),
                Err(e) => json_response(200, json!({ "error": e }).to_string()),
            }
        }
        (Method::Get, p) if p.starts_with("/photos/") => {
            let name = std::path::Path::new(&p["/photos/".len()..])
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
                .unwrap_or_default();
            let path = match qrz::photos_dir(app) {
                Ok(d) => d.join(&name),
                Err(e) => return error(500, &e),
            };
            match fs::read(&path) {
                Ok(bytes) => {
                    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
                    let mime = match ext.as_str() {
                        "png" => "image/png",
                        "gif" => "image/gif",
                        "webp" => "image/webp",
                        _ => "image/jpeg",
                    };
                    Response::from_data(bytes)
                        .with_header(header("Content-Type", mime))
                        .with_header(header("Cache-Control", "private, max-age=86400"))
                }
                Err(_) => error(404, "No such photo."),
            }
        }
        _ => error(404, "Not found."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_six_digits() {
        for _ in 0..50 {
            let c = new_code();
            assert_eq!(c.len(), 6);
            assert!(c.chars().all(|ch| ch.is_ascii_digit()));
        }
    }

    #[test]
    fn page_keeps_script_closed() {
        let inner = Inner::default();
        {
            let mut s = inner.store.lock().unwrap();
            s.version = 3;
            s.json = r#"{"nets":{"a":{"note":"</script><script>alert(1)</script>"}},"activeNetId":"a"}"#.into();
        }
        let html = page(&inner, true);
        let start = html.find("window.HAMNET_REMOTE").unwrap();
        let end = html[start..].find("</script>").unwrap();
        assert!(html[start..start + end].contains(r#"<\/script>"#));
        assert!(page(&inner, false).contains(r#"{"paired":false}"#));
    }
}
