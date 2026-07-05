use serde::Serialize;
use serde_json::{json, Value};

// HowLongToBeat has no official API. This mirrors what their site does
// (verified live 2026-07-05): GET /api/bleed/init?t=<ms> hands out
// {token, hpKey, hpVal}, then POST /api/bleed with those as x-auth-token /
// x-hp-key / x-hp-val headers (hpKey:hpVal also goes into the payload) runs
// the search. The endpoint name has rotated before (search → seek → s →
// bleed), so if "bleed" stops answering, the name is re-discovered from the
// site's JS chunks by looking for the telltale "/init?t=" call. Best-effort.

const SITE: &str = "https://howlongtobeat.com";
const KNOWN_BASE: &str = "bleed";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                  (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

/// Completion times in minutes (matching the library's playtimeMinutes unit).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HltbTimes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completionist: Option<u32>,
}

struct Tokens {
    token: String,
    hp_key: Option<String>,
    hp_val: Option<String>,
}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// Fetch a search token from /api/<base>/init.
async fn init_tokens(client: &reqwest::Client, base: &str) -> Option<Tokens> {
    let url = format!("{SITE}/api/{base}/init?t={}", now_ms());
    let v: Value = client
        .get(&url)
        .header("Referer", format!("{SITE}/"))
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .await
        .ok()?;
    Some(Tokens {
        token: v.get("token")?.as_str()?.to_string(),
        hp_key: v.get("hpKey").and_then(|k| k.as_str()).map(String::from),
        hp_val: v.get("hpVal").and_then(|k| k.as_str()).map(String::from),
    })
}

/// Re-discover the endpoint name from the site's JS chunks: the init call
/// (`fetch("/api/<name>/init?t=...")`) marks the search module.
async fn discover_base(client: &reqwest::Client) -> Option<String> {
    let html = client.get(SITE).send().await.ok()?.text().await.ok()?;
    let mut chunks = Vec::new();
    let mut rest = html.as_str();
    while let Some(pos) = rest.find("_next/static/chunks/") {
        let tail = &rest[pos..];
        let Some(end) = tail.find(".js") else { break };
        let path = &tail[..end + ".js".len()];
        if !path.contains('"') && !chunks.contains(&path.to_string()) {
            chunks.push(path.to_string());
        }
        rest = &tail[end..];
    }
    for path in chunks.iter().take(20) {
        let Ok(resp) = client.get(format!("{SITE}/{path}")).send().await else {
            continue;
        };
        let Ok(js) = resp.text().await else { continue };
        if let Some(p) = js.find("/init?t=") {
            let head = &js[..p];
            if let Some(a) = head.rfind("/api/") {
                let name = &head[a + "/api/".len()..];
                if !name.is_empty()
                    && name
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                {
                    return Some(name.to_string());
                }
            }
        }
    }
    None
}

async fn search(
    client: &reqwest::Client,
    base: &str,
    tokens: &Tokens,
    title: &str,
) -> Result<reqwest::Response, reqwest::Error> {
    let terms: Vec<&str> = title.split_whitespace().collect();
    let mut payload = json!({
        "searchType": "games",
        "searchTerms": terms,
        "searchPage": 1,
        "size": 20,
        "searchOptions": {
            "games": {
                "userId": 0,
                "platform": "",
                "sortCategory": "popular",
                "rangeCategory": "main",
                "rangeTime": { "min": null, "max": null },
                "gameplay": { "perspective": "", "flow": "", "genre": "", "difficulty": "" },
                "rangeYear": { "min": "", "max": "" },
                "modifier": ""
            },
            "users": { "sortCategory": "postcount" },
            "lists": { "sortCategory": "follows" },
            "filter": "",
            "sort": 0,
            "randomizer": 0
        },
        "useCache": true
    });
    let mut req = client
        .post(format!("{SITE}/api/{base}"))
        .header("Referer", format!("{SITE}/"))
        .header("Origin", SITE)
        .header("x-auth-token", &tokens.token);
    if let (Some(k), Some(v)) = (&tokens.hp_key, &tokens.hp_val) {
        payload[k.as_str()] = json!(v);
        req = req.header("x-hp-key", k).header("x-hp-val", v);
    }
    req.json(&payload).send().await
}

/// Loose title comparison: lowercase alphanumerics only.
fn norm(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

/// Seconds (HLTB's unit) to minutes; 0 means "no data" on HLTB.
fn to_minutes(v: Option<&Value>) -> Option<u32> {
    let secs = v?.as_u64()?;
    if secs == 0 {
        return None;
    }
    Some(((secs + 30) / 60) as u32)
}

/// Look a game up on HowLongToBeat. Ok(None) = HLTB has no entry (cacheable);
/// Err = transient/scrape failure (caller should retry another time).
#[tauri::command]
pub async fn hltb_search(title: String) -> Result<Option<HltbTimes>, String> {
    let title = title.trim();
    if title.is_empty() {
        return Ok(None);
    }
    let client = reqwest::Client::builder()
        .user_agent(UA)
        .build()
        .map_err(|e| e.to_string())?;

    // Known endpoint name first; if its init is gone, re-discover the name.
    let mut base = KNOWN_BASE.to_string();
    let mut tokens = init_tokens(&client, &base).await;
    if tokens.is_none() {
        base = discover_base(&client)
            .await
            .ok_or("Could not locate HowLongToBeat's search endpoint.")?;
        tokens = init_tokens(&client, &base).await;
    }
    let mut tok = tokens.ok_or("HowLongToBeat did not hand out a search token.")?;

    let mut resp = search(&client, &base, &tok, title)
        .await
        .map_err(|e| format!("HowLongToBeat request failed: {e}"))?;
    if resp.status() == reqwest::StatusCode::FORBIDDEN {
        // Token expired — refresh once and retry, like the site does.
        tok = init_tokens(&client, &base)
            .await
            .ok_or("HowLongToBeat search token refresh failed.")?;
        resp = search(&client, &base, &tok, title)
            .await
            .map_err(|e| format!("HowLongToBeat request failed: {e}"))?;
    }
    if !resp.status().is_success() {
        return Err(format!("HowLongToBeat returned HTTP {}.", resp.status()));
    }
    let v: Value = resp
        .json()
        .await
        .map_err(|e| format!("Could not parse HowLongToBeat's response: {e}"))?;

    let Some(data) = v.get("data").and_then(|d| d.as_array()) else {
        return Err("Unexpected HowLongToBeat response shape.".into());
    };
    // Exact (normalized) title match if there is one, else HLTB's top hit.
    let wanted = norm(title);
    let hit = data
        .iter()
        .find(|g| g.get("game_name").and_then(|n| n.as_str()).map(norm) == Some(wanted.clone()))
        .or_else(|| data.first());
    Ok(hit.map(|g| HltbTimes {
        main: to_minutes(g.get("comp_main")),
        extra: to_minutes(g.get("comp_plus")),
        completionist: to_minutes(g.get("comp_100")),
    }))
}
