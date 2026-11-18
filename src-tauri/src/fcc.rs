//! FCC license data for US callsigns, through two free lookup services built on the
//! FCC's own database: callook.info first (it keeps up with new licenses; HamDB.org
//! was found missing ones months old), then HamDB.org when callook.info can't answer
//! or doesn't know the callsign.
//! Used when there's no QRZ login, when QRZ doesn't know a callsign, and to add the
//! license class and expiration date when QRZ's answer doesn't include them.

use serde_json::Value;

const AGENT: &str = concat!("N8VCLNetLogger", env!("CARGO_PKG_VERSION"));

/// What the FCC has for a callsign. Names and addresses come in capitals, as licensed.
#[derive(Debug, Default, PartialEq)]
pub struct FccRecord {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub street: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
    /// One letter: E, A, G, T, N, P (empty for club stations)
    pub license_class: Option<String>,
    /// YYYY-MM-DD
    pub expires: Option<String>,
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent(AGENT)
        .build()
}

/// The part of a callsign the FCC would have: "W1AW" from "W1AW/M" or "VE3/W1AW",
/// and None for one that isn't a US callsign at all.
pub fn us_base_call(callsign: &str) -> Option<String> {
    let call = callsign.trim().to_uppercase();
    call.split('/')
        .filter(|part| is_us_call(part))
        .max_by_key(|part| part.len())
        .map(str::to_string)
}

fn is_us_call(c: &str) -> bool {
    let b = c.as_bytes();
    if b.len() < 3 || b.len() > 6 || !b.iter().all(|x| x.is_ascii_alphanumeric()) {
        return false;
    }
    let prefix_ok = matches!(b[0], b'K' | b'N' | b'W') || (b[0] == b'A' && (b'A'..=b'L').contains(&b[1]));
    let digits = b.iter().filter(|x| x.is_ascii_digit()).count();
    prefix_ok && digits == 1 && b.last().map_or(false, |x| x.is_ascii_alphabetic())
}

/// "02/26/2031" -> "2031-02-26"; a date already in that form is kept.
pub fn iso_date(s: &str) -> Option<String> {
    let s = s.trim();
    let mut it = s.split(if s.contains('-') { '-' } else { '/' });
    let (a, b, c) = (it.next()?, it.next()?, it.next()?);
    let (m, d, y) = if s.contains('-') { (b, c, a) } else { (a, b, c) };
    let (m, d, y): (u32, u32, u32) = (m.parse().ok()?, d.parse().ok()?, y.parse().ok()?);
    (it.next().is_none() && (1..=12).contains(&m) && (1..=31).contains(&d) && y >= 1900)
        .then(|| format!("{:04}-{:02}-{:02}", y, m, d))
}

/// "EXTRA" -> "E", "e" -> "E"; the services spell the class differently.
pub fn class_letter(s: &str) -> Option<String> {
    let s = s.trim().to_uppercase();
    let letter = match s.as_str() {
        "" => return None,
        "EXTRA" | "AMATEUR EXTRA" => "E",
        "ADVANCED" => "A",
        "GENERAL" => "G",
        "TECHNICIAN" => "T",
        "TECHNICIAN PLUS" => "P",
        "NOVICE" => "N",
        other if other.len() == 1 => other,
        _ => return None,
    };
    Some(letter.to_string())
}

fn text(v: &Value, key: &str) -> Option<String> {
    let s = v.get(key)?.as_str()?.trim();
    (!s.is_empty() && s != "NOT_FOUND").then(|| s.to_string())
}

/// HamDB's answer: Ok(None) when the callsign isn't licensed.
fn parse_hamdb(body: &str) -> Result<Option<FccRecord>, String> {
    let v: Value = serde_json::from_str(body).map_err(|e| format!("HamDB sent something unexpected: {}", e))?;
    let c = v.pointer("/hamdb/callsign").ok_or("HamDB sent something unexpected.")?;
    let status = v.pointer("/hamdb/messages/status").and_then(Value::as_str).unwrap_or("");
    if status == "NOT_FOUND" || text(c, "call").is_none() {
        return Ok(None);
    }
    Ok(Some(FccRecord {
        first_name: text(c, "fname"),
        last_name: text(c, "name"),
        street: text(c, "addr1"),
        city: text(c, "addr2"),
        state: text(c, "state"),
        zip: text(c, "zip"),
        license_class: text(c, "class").and_then(|s| class_letter(&s)),
        expires: text(c, "expires").and_then(|s| iso_date(&s)),
    }))
}

/// callook.info's answer. Its name is one string ("JOSHUA D WELLS") and its second
/// address line is "HARTFORD, CT 06103", so both are taken apart here.
fn parse_callook(body: &str) -> Result<Option<FccRecord>, String> {
    let v: Value = serde_json::from_str(body).map_err(|e| format!("callook.info sent something unexpected: {}", e))?;
    match v.get("status").and_then(Value::as_str) {
        Some("VALID") => {}
        Some("INVALID") => return Ok(None),
        _ => return Err("callook.info couldn't answer just now.".into()),
    }
    let mut rec = FccRecord::default();
    let name = text(&v, "name").unwrap_or_default();
    if v.get("type").and_then(Value::as_str) == Some("PERSON") {
        let mut words: Vec<&str> = name.split_whitespace().collect();
        while words.len() > 2 && matches!(words[words.len() - 1].trim_end_matches('.'), "JR" | "SR" | "II" | "III" | "IV") {
            words.pop();
        }
        if words.len() >= 2 {
            rec.last_name = words.pop().map(str::to_string);
            rec.first_name = Some(words.join(" "));
        } else {
            rec.last_name = (!name.is_empty()).then(|| name.clone());
        }
    } else if !name.is_empty() {
        rec.last_name = Some(name);
    }
    if let Some(addr) = v.get("address") {
        rec.street = text(addr, "line1");
        if let Some(line2) = text(addr, "line2") {
            // "HARTFORD, CT 06103"
            if let Some((city, rest)) = line2.rsplit_once(',') {
                let mut parts = rest.split_whitespace();
                rec.city = Some(city.trim().to_string()).filter(|s| !s.is_empty());
                rec.state = parts.next().map(str::to_string);
                rec.zip = parts.next().map(str::to_string);
            } else {
                rec.city = Some(line2);
            }
        }
    }
    rec.license_class = v.pointer("/current/operClass").and_then(Value::as_str).and_then(class_letter);
    rec.expires = v.pointer("/otherInfo/expiryDate").and_then(Value::as_str).and_then(iso_date);
    Ok(Some(rec))
}

fn get(url: &str) -> Result<String, String> {
    agent().get(url).call().map_err(|e| e.to_string())?.into_string().map_err(|e| e.to_string())
}

/// Looks a callsign up in the FCC data. Ok(None): not a US callsign, or not licensed.
/// Blocking; call it off the UI thread.
pub fn lookup(callsign: &str) -> Result<Option<FccRecord>, String> {
    let Some(call) = us_base_call(callsign) else { return Ok(None) };
    let callook = get(&format!("https://callook.info/{}/json", call)).and_then(|b| parse_callook(&b));
    if let Ok(Some(found)) = callook {
        return Ok(Some(found));
    }
    let hamdb = get(&format!("https://api.hamdb.org/v1/{}/json/N8VCLNetLogger", call)).and_then(|b| parse_hamdb(&b));
    match (callook, hamdb) {
        (_, Ok(Some(found))) => Ok(Some(found)),
        // Not found by whichever answered: not licensed (or too new for both)
        (Ok(None), _) | (_, Ok(None)) => Ok(None),
        (Err(first), Err(second)) => Err(format!("couldn't reach the FCC lookup (callook.info: {}; HamDB: {})", first, second)),
        (Ok(Some(_)), _) => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]   // reaches the internet: cargo test live_fcc -- --ignored
    fn live_fcc_lookup() {
        let r = lookup("AF5ID").unwrap().unwrap();
        assert_eq!(r.last_name.as_deref(), Some("WELLS"));
        assert!(r.expires.is_some());
        assert_eq!(lookup("ZZ9ZZZ").unwrap(), None);
        // Licensed in 2026: on callook.info, missing from HamDB
        let new = lookup("AL0R").unwrap().unwrap();
        assert_eq!(new.last_name.as_deref(), Some("JONES"));
        assert_eq!(new.first_name.as_deref(), Some("MATTHEW R"));
        assert_eq!(new.city.as_deref(), Some("VALLEY"));
        assert_eq!(new.license_class.as_deref(), Some("E"));
        assert_eq!(lookup("VE3ABC").unwrap(), None);
        let c = get("https://callook.info/AF5ID/json").and_then(|b| parse_callook(&b)).unwrap().unwrap();
        assert_eq!(c.last_name.as_deref(), Some("WELLS"));
    }

    #[test]
    fn finds_the_us_part_of_a_callsign() {
        assert_eq!(us_base_call("w1aw").as_deref(), Some("W1AW"));
        assert_eq!(us_base_call("W1AW/M").as_deref(), Some("W1AW"));
        assert_eq!(us_base_call("VE3/AE5EI").as_deref(), Some("AE5EI"));
        assert_eq!(us_base_call("KD0ABC").as_deref(), Some("KD0ABC"));
        assert_eq!(us_base_call("AL7XX").as_deref(), Some("AL7XX"));
        assert_eq!(us_base_call("VE3ABC"), None);
        assert_eq!(us_base_call("AM1ABC"), None);   // AM is Spain
        assert_eq!(us_base_call("G4ABC"), None);
        assert_eq!(us_base_call("NEW1CALL"), None);
    }

    #[test]
    fn dates_and_classes() {
        assert_eq!(iso_date("02/26/2031").as_deref(), Some("2031-02-26"));
        assert_eq!(iso_date("7/1/2032").as_deref(), Some("2032-07-01"));
        assert_eq!(iso_date("2031-02-26").as_deref(), Some("2031-02-26"));
        assert_eq!(iso_date("NOT_FOUND"), None);
        assert_eq!(iso_date("0000-00-00"), None);
        assert_eq!(class_letter("EXTRA").as_deref(), Some("E"));
        assert_eq!(class_letter("g").as_deref(), Some("G"));
        assert_eq!(class_letter(""), None);
    }

    #[test]
    fn reads_hamdb() {
        let body = r#"{"hamdb":{"version":"1","callsign":{"call":"AF5ID","class":"E","expires":"07/11/2032","status":"A",
            "fname":"JOSHUA","mi":"D","name":"WELLS","suffix":"","addr1":"101 Pearl Street Apt 604","addr2":"Hartford",
            "state":"CT","zip":"06103","country":"United States"},"messages":{"status":"OK"}}}"#;
        let r = parse_hamdb(body).unwrap().unwrap();
        assert_eq!(r.first_name.as_deref(), Some("JOSHUA"));
        assert_eq!(r.last_name.as_deref(), Some("WELLS"));
        assert_eq!(r.city.as_deref(), Some("Hartford"));
        assert_eq!(r.license_class.as_deref(), Some("E"));
        assert_eq!(r.expires.as_deref(), Some("2032-07-11"));
        let missing = r#"{"hamdb":{"version":"1","callsign":{"call":"NOT_FOUND","class":"NOT_FOUND"},"messages":{"status":"NOT_FOUND"}}}"#;
        assert_eq!(parse_hamdb(missing).unwrap(), None);
        assert!(parse_hamdb("<html>").is_err());
    }

    #[test]
    fn reads_callook() {
        let body = r#"{"status":"VALID","type":"PERSON","current":{"callsign":"AF5ID","operClass":"EXTRA"},
            "name":"JOSHUA D WELLS JR","address":{"line1":"101 PEARL STREET APT 604","line2":"HARTFORD, CT 06103","attn":""},
            "otherInfo":{"expiryDate":"07/11/2032"}}"#;
        let r = parse_callook(body).unwrap().unwrap();
        assert_eq!(r.first_name.as_deref(), Some("JOSHUA D"));
        assert_eq!(r.last_name.as_deref(), Some("WELLS"));
        assert_eq!(r.city.as_deref(), Some("HARTFORD"));
        assert_eq!(r.state.as_deref(), Some("CT"));
        assert_eq!(r.zip.as_deref(), Some("06103"));
        assert_eq!(r.license_class.as_deref(), Some("E"));
        assert_eq!(r.expires.as_deref(), Some("2032-07-11"));
        assert_eq!(parse_callook(r#"{"status":"INVALID"}"#).unwrap(), None);
        assert!(parse_callook(r#"{"status":"UPDATING"}"#).is_err());
        let club = r#"{"status":"VALID","type":"CLUB","current":{"operClass":""},"name":"ARRL HQ OPERATORS CLUB",
            "address":{"line1":"225 MAIN ST","line2":"NEWINGTON, CT 06111"},"otherInfo":{"expiryDate":"02/26/2031"}}"#;
        let c = parse_callook(club).unwrap().unwrap();
        assert_eq!(c.first_name, None);
        assert_eq!(c.last_name.as_deref(), Some("ARRL HQ OPERATORS CLUB"));
        assert_eq!(c.license_class, None);
    }
}
