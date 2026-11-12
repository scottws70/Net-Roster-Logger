//! Current weather for the header tiles, from a Weather Underground personal station
//! (needs the user's WU API key) or a National Weather Service station (no key).
//! Everything is returned in US units: °F, %, mph, inHg, inches.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

const WU_URL: &str = "https://api.weather.com/v2/pws/observations/current";
const NWS_URL: &str = "https://api.weather.gov/stations";
// NWS asks every client to identify itself
const USER_AGENT: &str = concat!(
    "N8VCL-Net-Logger/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/scottws70/Net-Roster-Logger)"
);

#[derive(Serialize, Deserialize, Default)]
struct WeatherKeys {
    wu_api_key: String,
}

#[derive(Serialize, Default, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Weather {
    station: String,
    /// Observation time, seconds since 1970 (UTC)
    observed: Option<i64>,
    temp_f: Option<f64>,
    humidity: Option<f64>,
    wind_dir: Option<f64>,
    wind_mph: Option<f64>,
    gust_mph: Option<f64>,
    pressure_in: Option<f64>,
    precip_in: Option<f64>,
    /// What the precip figure covers: "today" (WU, since local midnight) or "24h" (NWS)
    precip_span: String,
}

fn keys_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("weather.json"))
}

fn load_keys(app: &AppHandle) -> WeatherKeys {
    keys_path(app)
        .ok()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(20))
        .user_agent(USER_AGENT)
        .build()
}

fn read_json(r: ureq::Response) -> Result<Value, String> {
    let text = r.into_string().map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("Unexpected reply: {}", e))
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Seconds since 1970 for an ISO time like "2026-11-12T05:53:00+00:00" or "...Z".
fn parse_utc(ts: &str) -> Option<i64> {
    let num = |a: usize, b: usize| ts.get(a..b)?.parse::<i64>().ok();
    let (y, m, d) = (num(0, 4)?, num(5, 7)?, num(8, 10)?);
    let (hh, mm, ss) = (num(11, 13)?, num(14, 16)?, num(17, 19)?);
    // Days from 1970-01-01 to y-m-d (proleptic Gregorian)
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    let mut secs = days * 86400 + hh * 3600 + mm * 60 + ss;
    // Offset suffix like +00:00 / -05:00 (NWS sends +00:00)
    if let Some(off) = ts.get(19..).filter(|o| o.len() >= 6 && (o.starts_with('+') || o.starts_with('-'))) {
        let sign = if off.starts_with('-') { -1 } else { 1 };
        let oh: i64 = off.get(1..3)?.parse().ok()?;
        let om: i64 = off.get(4..6)?.parse().ok()?;
        secs -= sign * (oh * 3600 + om * 60);
    }
    Some(secs)
}

/// Reads a Weather Underground "current observation" response.
fn parse_wu(station: &str, v: &Value) -> Result<Weather, String> {
    let obs = v["observations"]
        .get(0)
        .ok_or_else(|| format!("Weather Underground has no current data for {}.", station))?;
    let imp = &obs["imperial"];
    Ok(Weather {
        station: obs["stationID"].as_str().unwrap_or(station).to_string(),
        observed: obs["obsTimeUtc"].as_str().and_then(parse_utc),
        temp_f: imp["temp"].as_f64(),
        humidity: obs["humidity"].as_f64(),
        wind_dir: obs["winddir"].as_f64(),
        wind_mph: imp["windSpeed"].as_f64(),
        gust_mph: imp["windGust"].as_f64(),
        pressure_in: imp["pressure"].as_f64(),
        precip_in: imp["precipTotal"].as_f64(),
        precip_span: "today".into(),
    })
}

/// Value of an NWS quantity like {"unitCode": "wmoUnit:degC", "value": 3.9}, converted to US units.
fn nws_value(q: &Value) -> Option<f64> {
    let v = q["value"].as_f64()?;
    let unit = q["unitCode"].as_str().unwrap_or("");
    Some(match unit.rsplit(':').next().unwrap_or("") {
        "degC" => v * 9.0 / 5.0 + 32.0,
        "km_h-1" => v / 1.609344,
        "m_s-1" => v * 2.236936,
        "Pa" => v / 3386.389,
        "mm" => v / 25.4,
        _ => v,
    })
}

/// Reads an NWS observation list (newest first). Each reading comes from the newest
/// report that has it; precip is the sum of the station's hourly rain amounts over 24 hours.
fn parse_nws(station: &str, v: &Value, now: i64) -> Result<Weather, String> {
    let feats = v["features"].as_array().cloned().unwrap_or_default();
    let props: Vec<&Value> = feats.iter().map(|f| &f["properties"]).collect();
    if props.is_empty() {
        return Err(format!("NWS has no recent reports from {}.", station));
    }
    let first = |key: &str| props.iter().find_map(|p| nws_value(&p[key]));

    // Routine hourly reports all share one minute past the hour (e.g. :53); extra
    // "special" reports in between repeat part of the hour's rain, so only routine ones count.
    let recent: Vec<(&Value, i64)> = props
        .iter()
        .filter_map(|p| Some((*p, parse_utc(p["timestamp"].as_str()?)?)))
        .filter(|(_, t)| now - t <= 24 * 3600)
        .collect();
    let mut minute_counts = [0u32; 60];
    for (_, t) in &recent {
        minute_counts[(t.rem_euclid(3600) / 60) as usize] += 1;
    }
    let routine_minute = (0..60).max_by_key(|&m| minute_counts[m]).unwrap_or(0) as i64;
    let precip: f64 = recent
        .iter()
        .filter(|(_, t)| t.rem_euclid(3600) / 60 == routine_minute)
        .filter_map(|(p, _)| nws_value(&p["precipitationLastHour"]))
        .fold(0.0, |a, b| a + b);

    Ok(Weather {
        station: station.to_uppercase(),
        observed: props[0]["timestamp"].as_str().and_then(parse_utc),
        temp_f: first("temperature"),
        humidity: first("relativeHumidity"),
        wind_dir: first("windDirection"),
        wind_mph: first("windSpeed"),
        // A gust is only meaningful from the newest report; an older one is long over
        gust_mph: nws_value(&props[0]["windGust"]),
        pressure_in: first("barometricPressure"),
        precip_in: if recent.is_empty() { None } else { Some(precip) },
        precip_span: "24h".into(),
    })
}

fn fetch_blocking(key: &str, source: &str, station: &str) -> Result<Weather, String> {
    let station = station.trim();
    if station.is_empty() {
        return Err("Enter a station ID in Settings → Weather.".into());
    }
    match source {
        "wu" => {
            if key.is_empty() {
                return Err("Save your Weather Underground API key in Settings → Weather.".into());
            }
            let resp = agent()
                .get(WU_URL)
                .query("stationId", station)
                .query("format", "json")
                .query("units", "e")
                .query("numericPrecision", "decimal")
                .query("apiKey", key)
                .call();
            match resp {
                Ok(r) if r.status() == 204 => {
                    Err(format!("{} isn't reporting to Weather Underground right now.", station))
                }
                Ok(r) => parse_wu(station, &read_json(r)?),
                Err(ureq::Error::Status(401, _)) => Err("Weather Underground didn't accept the API key.".into()),
                Err(ureq::Error::Status(404, _)) => Err(format!("Weather Underground doesn't know station {}.", station)),
                Err(e) => Err(format!("Couldn't reach Weather Underground: {}", e)),
            }
        }
        "nws" => {
            let url = format!("{}/{}/observations", NWS_URL, station.to_uppercase());
            let resp = agent()
                .get(&url)
                .set("Accept", "application/geo+json")
                .query("limit", "100")
                .call();
            match resp {
                Ok(r) => parse_nws(station, &read_json(r)?, now_secs()),
                Err(ureq::Error::Status(404, _)) => Err(format!("NWS doesn't know station {}.", station)),
                Err(e) => Err(format!("Couldn't reach the National Weather Service: {}", e)),
            }
        }
        _ => Err("Weather is turned off for this net.".into()),
    }
}

#[tauri::command]
pub fn weather_has_wu_key(app: AppHandle) -> bool {
    !load_keys(&app).wu_api_key.is_empty()
}

/// Saves (or, with an empty key, removes) the Weather Underground API key on this computer.
#[tauri::command]
pub fn weather_save_wu_key(app: AppHandle, key: String) -> Result<(), String> {
    let path = keys_path(&app)?;
    let keys = WeatherKeys { wu_api_key: key.trim().to_string() };
    if keys.wu_api_key.is_empty() {
        if path.exists() {
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    let json = serde_json::to_string(&keys).map_err(|e| e.to_string())?;
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    use std::io::Write;
    opts.open(path)
        .and_then(|mut f| f.write_all(json.as_bytes()))
        .map_err(|e| e.to_string())
}

/// Current conditions from `source` ("wu" or "nws") for `station`.
#[tauri::command]
pub async fn weather_fetch(app: AppHandle, source: String, station: String) -> Result<Weather, String> {
    let key = load_keys(&app).wu_api_key;
    tauri::async_runtime::spawn_blocking(move || fetch_blocking(&key, &source, &station))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_times() {
        assert_eq!(parse_utc("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_utc("2026-11-12T05:53:00+00:00"), Some(1794462780));
        assert_eq!(parse_utc("2026-11-12T00:53:00-05:00"), Some(1794462780));
    }

    #[test]
    fn reads_wu() {
        let v = json!({"observations": [{"stationID": "KNELINCO123", "obsTimeUtc": "2026-11-12T05:50:00Z",
            "humidity": 81.0, "winddir": 315.0,
            "imperial": {"temp": 41.2, "windSpeed": 7.0, "windGust": 12.0, "pressure": 30.05, "precipTotal": 0.12}}]});
        let w = parse_wu("knelinco123", &v).unwrap();
        assert_eq!(w.station, "KNELINCO123");
        assert_eq!(w.temp_f, Some(41.2));
        assert_eq!(w.precip_in, Some(0.12));
        assert_eq!(w.precip_span, "today");
        assert!(parse_wu("X", &json!({"observations": []})).is_err());
    }

    #[test]
    fn reads_nws_and_sums_hourly_rain() {
        let obs = |ts: &str, temp: Value, rain_mm: Value| json!({"properties": {
            "timestamp": ts,
            "temperature": {"unitCode": "wmoUnit:degC", "value": temp},
            "relativeHumidity": {"unitCode": "wmoUnit:percent", "value": 80.0},
            "windDirection": {"unitCode": "wmoUnit:degree_(angle)", "value": 320.0},
            "windSpeed": {"unitCode": "wmoUnit:km_h-1", "value": 16.09344},
            "windGust": {"unitCode": "wmoUnit:km_h-1", "value": null},
            "barometricPressure": {"unitCode": "wmoUnit:Pa", "value": 101592.0},
            "precipitationLastHour": {"unitCode": "wmoUnit:mm", "value": rain_mm}}});
        let v = json!({"features": [
            obs("2026-11-12T05:53:00+00:00", json!(null), json!(2.54)),   // newest has no temp
            obs("2026-11-12T05:20:00+00:00", json!(5.0), json!(1.0)),     // special: part of the 05:53 hour
            obs("2026-11-12T04:53:00+00:00", json!(4.0), json!(2.54)),
            obs("2026-11-12T03:53:00+00:00", json!(4.0), json!(null)),
            obs("2026-11-11T02:53:00+00:00", json!(4.0), json!(25.4)),    // over 24h old
        ]});
        let now = parse_utc("2026-11-12T06:00:00Z").unwrap();
        let w = parse_nws("klnk", &v, now).unwrap();
        assert_eq!(w.station, "KLNK");
        assert_eq!(w.temp_f, Some(5.0 * 9.0 / 5.0 + 32.0));   // newest report that has a temperature
        assert!((w.wind_mph.unwrap() - 10.0).abs() < 1e-9);
        assert_eq!(w.gust_mph, None);
        assert!((w.pressure_in.unwrap() - 30.0).abs() < 0.01);
        assert!((w.precip_in.unwrap() - 0.2).abs() < 1e-9);   // 0.1 + 0.1, special and old report skipped
        assert_eq!(w.precip_span, "24h");
    }

    /// Live check against api.weather.gov: `cargo test --lib -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn live_nws() {
        let w = fetch_blocking("", "nws", "KLNK").unwrap();
        println!("{:?}", w);
        assert!(w.temp_f.is_some() && w.observed.is_some());
    }
}
