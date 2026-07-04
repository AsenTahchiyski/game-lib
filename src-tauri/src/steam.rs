use serde::{Deserialize, Serialize};

// --- Steam Web API response shapes (only the fields we use) ---

#[derive(Deserialize)]
struct OwnedGamesResponse {
    response: OwnedGames,
}

#[derive(Deserialize)]
struct OwnedGames {
    #[serde(default)]
    games: Vec<OwnedGame>,
}

#[derive(Deserialize)]
struct OwnedGame {
    appid: u32,
    #[serde(default)]
    name: String,
    #[serde(default)]
    playtime_forever: u64, // minutes
}

/// One game returned to the frontend for merging into the library.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamGame {
    pub appid: u32,
    pub name: String,
    pub playtime_minutes: u64,
    pub metacritic: Option<u32>,   // 0-100, from the Steam store page
    pub store_rating: Option<u32>, // 0-100, Steam review % positive
    pub release_timestamp: Option<i64>, // unix seconds; the frontend turns it into a date
    pub wishlist: bool,            // true => from the wishlist, not owned
}

/// Appids on the user's Steam wishlist via the official IWishlistService. Public
/// wishlist required; best-effort (empty on error/private).
async fn fetch_wishlist_appids(client: &reqwest::Client, steam_id: &str) -> Vec<u32> {
    let url = format!(
        "https://api.steampowered.com/IWishlistService/GetWishlist/v1/?steamid={steam_id}"
    );
    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let v: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    v.get("response")
        .and_then(|r| r.get("items"))
        .and_then(|i| i.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|it| it.get("appid").and_then(|a| a.as_u64()).map(|n| n as u32))
                .collect()
        })
        .unwrap_or_default()
}

/// Per-app info from the bulk store catalog endpoint.
struct StoreItem {
    name: Option<String>,
    percent_positive: Option<u32>,
    release_timestamp: Option<i64>,
}

/// Names, review % and release dates via IStoreBrowseService/GetItems — one
/// request per 200 apps instead of one per app, so wishlist entries aren't
/// dropped by store rate limits. Best-effort: failed chunks are just skipped.
async fn fetch_store_items(
    client: &reqwest::Client,
    appids: &[u32],
) -> std::collections::HashMap<u32, StoreItem> {
    let mut out = std::collections::HashMap::new();
    for chunk in appids.chunks(200) {
        let ids: Vec<serde_json::Value> =
            chunk.iter().map(|a| serde_json::json!({ "appid": a })).collect();
        let input = serde_json::json!({
            "ids": ids,
            "context": { "language": "english", "country_code": "US", "steam_realm": 1 },
            "data_request": { "include_release": true, "include_reviews": true },
        });
        let resp = match client
            .get("https://api.steampowered.com/IStoreBrowseService/GetItems/v1/")
            .query(&[("input_json", input.to_string())])
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => continue,
        };
        let v: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => continue,
        };
        let Some(items) = v.pointer("/response/store_items").and_then(|i| i.as_array()) else {
            continue;
        };
        for it in items {
            let Some(appid) = it.get("appid").and_then(|a| a.as_u64()).map(|n| n as u32) else {
                continue;
            };
            out.insert(
                appid,
                StoreItem {
                    name: it.get("name").and_then(|n| n.as_str()).map(String::from),
                    // 0 means "no reviews yet" (e.g. unreleased) — treat as unknown.
                    percent_positive: it
                        .pointer("/reviews/summary_filtered/percent_positive")
                        .and_then(|p| p.as_u64())
                        .filter(|&n| n > 0)
                        .map(|n| n as u32),
                    // 0 means TBA/unreleased — treat as unknown.
                    release_timestamp: it
                        .pointer("/release/steam_release_date")
                        .and_then(|t| t.as_i64())
                        .filter(|&t| t > 0),
                },
            );
        }
    }
    out
}

/// Metacritic score for an app from the store's appdetails endpoint. Best
/// effort — returns None on any error or missing data.
async fn fetch_metacritic(client: &reqwest::Client, appid: u32) -> Option<u32> {
    let url =
        format!("https://store.steampowered.com/api/appdetails?appids={appid}&filters=metacritic");
    let v: serde_json::Value = client.get(&url).send().await.ok()?.json().await.ok()?;
    let key = appid.to_string();
    v.get(key.as_str())?
        .get("data")?
        .get("metacritic")?
        .get("score")?
        .as_u64()
        .filter(|&n| n > 0)
        .map(|n| n as u32)
}

/// Fetch Metacritic scores for many appids with bounded concurrency. Failures
/// are swallowed (the game just gets no score), and the appdetails endpoint is
/// rate-limited, so coverage may be partial.
async fn fetch_metacritics(
    client: &reqwest::Client,
    appids: Vec<u32>,
) -> std::collections::HashMap<u32, Option<u32>> {
    const CONCURRENCY: usize = 12;
    let mut out = std::collections::HashMap::new();
    let mut iter = appids.into_iter();
    let mut set = tokio::task::JoinSet::new();

    let spawn_next = |set: &mut tokio::task::JoinSet<(u32, Option<u32>)>,
                      it: &mut std::vec::IntoIter<u32>| {
        if let Some(id) = it.next() {
            let c = client.clone();
            set.spawn(async move { (id, fetch_metacritic(&c, id).await) });
        }
    };

    for _ in 0..CONCURRENCY {
        spawn_next(&mut set, &mut iter);
    }
    while let Some(res) = set.join_next().await {
        if let Ok((id, score)) = res {
            out.insert(id, score);
        }
        spawn_next(&mut set, &mut iter);
    }
    out
}

/// Fetch the user's owned Steam games + playtime via the official Web API.
/// Runs through Rust's HTTP stack, so there's no browser CORS restriction.
///
/// Requires a public profile (game details set to public) — otherwise Steam
/// returns an empty list even with a valid key.
#[tauri::command]
pub async fn sync_steam(api_key: String, steam_id: String) -> Result<Vec<SteamGame>, String> {
    let api_key = api_key.trim();
    let steam_id = steam_id.trim();
    if api_key.is_empty() || steam_id.is_empty() {
        return Err("Steam API key and SteamID are both required.".into());
    }

    let url = format!(
        "https://api.steampowered.com/IPlayerService/GetOwnedGames/v1/\
         ?key={api_key}&steamid={steam_id}\
         &include_appinfo=1&include_played_free_games=1&format=json"
    );

    let resp = reqwest::Client::new()
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Steam request failed: {e}"))?;

    let status = resp.status();
    if status == reqwest::StatusCode::FORBIDDEN || status == reqwest::StatusCode::UNAUTHORIZED {
        return Err("Steam rejected the request (HTTP 401/403) — check your API key.".into());
    }
    if !status.is_success() {
        return Err(format!("Steam API returned HTTP {status}."));
    }

    let parsed: OwnedGamesResponse = resp.json().await.map_err(|e| {
        format!("Could not parse Steam's response ({e}). Check the API key and SteamID.")
    })?;

    let mut games: Vec<SteamGame> = parsed
        .response
        .games
        .into_iter()
        .map(|g| SteamGame {
            appid: g.appid,
            name: g.name,
            playtime_minutes: g.playtime_forever,
            metacritic: None,
            store_rating: None,
            release_timestamp: None,
            wishlist: false,
        })
        .collect();

    // Wishlist appids (best-effort; needs a public wishlist), minus owned.
    let client = reqwest::Client::new();
    let owned: std::collections::HashSet<u32> = games.iter().map(|g| g.appid).collect();
    let wishlist: Vec<u32> = fetch_wishlist_appids(&client, steam_id)
        .await
        .into_iter()
        .filter(|a| !owned.contains(a))
        .collect();

    // One bulk store lookup for everything: review % + release date for owned
    // games, plus the names for wishlist entries.
    let all_ids: Vec<u32> = games
        .iter()
        .map(|g| g.appid)
        .chain(wishlist.iter().copied())
        .collect();
    let items = fetch_store_items(&client, &all_ids).await;
    for g in &mut games {
        if let Some(it) = items.get(&g.appid) {
            g.store_rating = it.percent_positive;
            g.release_timestamp = it.release_timestamp;
        }
    }
    for appid in wishlist {
        let it = items.get(&appid);
        games.push(SteamGame {
            appid,
            // Empty name => the frontend labels it "Steam app <id>". Still
            // better than silently dropping the wishlist entry.
            name: it.and_then(|i| i.name.clone()).unwrap_or_default(),
            playtime_minutes: 0,
            metacritic: None,
            store_rating: it.and_then(|i| i.percent_positive),
            release_timestamp: it.and_then(|i| i.release_timestamp),
            wishlist: true,
        });
    }

    // Metacritic still needs the per-app appdetails endpoint (best-effort;
    // partial on rate limits).
    let metas = fetch_metacritics(&client, games.iter().map(|g| g.appid).collect()).await;
    for g in &mut games {
        g.metacritic = metas.get(&g.appid).copied().flatten();
    }

    Ok(games)
}
