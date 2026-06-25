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

/// Name (and Metacritic) for an app from the store appdetails endpoint.
async fn fetch_app_basic(client: &reqwest::Client, appid: u32) -> Option<(String, Option<u32>)> {
    let url = format!(
        "https://store.steampowered.com/api/appdetails?appids={appid}&filters=basic,metacritic"
    );
    let v: serde_json::Value = client.get(&url).send().await.ok()?.json().await.ok()?;
    let data = v.get(appid.to_string().as_str())?.get("data")?;
    let name = data.get("name")?.as_str()?.to_string();
    let metacritic = data
        .get("metacritic")
        .and_then(|m| m.get("score"))
        .and_then(|s| s.as_u64())
        .map(|n| n as u32);
    Some((name, metacritic))
}

/// Resolve wishlist appids into games (name + cover + Metacritic), skipping any
/// the user already owns. Bounded concurrency, best-effort.
async fn fetch_wishlist_games(
    client: &reqwest::Client,
    steam_id: &str,
    owned: &std::collections::HashSet<u32>,
) -> Vec<SteamGame> {
    let appids: Vec<u32> = fetch_wishlist_appids(client, steam_id)
        .await
        .into_iter()
        .filter(|a| !owned.contains(a))
        .collect();

    const CONCURRENCY: usize = 12;
    let mut out = Vec::new();
    let mut iter = appids.into_iter();
    let mut set = tokio::task::JoinSet::new();
    let spawn_next = |set: &mut tokio::task::JoinSet<(u32, Option<(String, Option<u32>)>)>,
                      it: &mut std::vec::IntoIter<u32>| {
        if let Some(id) = it.next() {
            let c = client.clone();
            set.spawn(async move { (id, fetch_app_basic(&c, id).await) });
        }
    };
    for _ in 0..CONCURRENCY {
        spawn_next(&mut set, &mut iter);
    }
    while let Some(res) = set.join_next().await {
        if let Ok((appid, Some((name, metacritic)))) = res {
            out.push(SteamGame {
                appid,
                name,
                playtime_minutes: 0,
                metacritic,
                store_rating: None,
                wishlist: true,
            });
        }
        spawn_next(&mut set, &mut iter);
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
        .map(|n| n as u32)
}

/// Steam's own rating (% positive reviews) from the appreviews endpoint.
async fn fetch_store_rating(client: &reqwest::Client, appid: u32) -> Option<u32> {
    let url = format!(
        "https://store.steampowered.com/appreviews/{appid}\
         ?json=1&language=all&num_per_page=0&purchase_type=all"
    );
    let v: serde_json::Value = client.get(&url).send().await.ok()?.json().await.ok()?;
    let q = v.get("query_summary")?;
    let positive = q.get("total_positive")?.as_u64()?;
    let total = q.get("total_reviews")?.as_u64()?;
    if total == 0 {
        return None;
    }
    Some(((positive as f64 / total as f64) * 100.0).round() as u32)
}

/// Fetch Metacritic + Steam review ratings for many appids with bounded
/// concurrency. Failures are swallowed (the game just gets no rating), and the
/// Steam store endpoints are rate-limited, so coverage may be partial.
async fn fetch_ratings(
    client: &reqwest::Client,
    appids: Vec<u32>,
) -> std::collections::HashMap<u32, (Option<u32>, Option<u32>)> {
    const CONCURRENCY: usize = 12;
    let mut out = std::collections::HashMap::new();
    let mut iter = appids.into_iter();
    let mut set = tokio::task::JoinSet::new();

    let spawn_next = |set: &mut tokio::task::JoinSet<(u32, (Option<u32>, Option<u32>))>,
                      it: &mut std::vec::IntoIter<u32>| {
        if let Some(id) = it.next() {
            let c = client.clone();
            set.spawn(async move {
                (id, (fetch_metacritic(&c, id).await, fetch_store_rating(&c, id).await))
            });
        }
    };

    for _ in 0..CONCURRENCY {
        spawn_next(&mut set, &mut iter);
    }
    while let Some(res) = set.join_next().await {
        if let Ok((id, ratings)) = res {
            out.insert(id, ratings);
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
            wishlist: false,
        })
        .collect();

    // Enrich with ratings (best-effort; partial on rate limits).
    let client = reqwest::Client::new();
    let ratings = fetch_ratings(&client, games.iter().map(|g| g.appid).collect()).await;
    for g in &mut games {
        if let Some((metacritic, store_rating)) = ratings.get(&g.appid) {
            g.metacritic = *metacritic;
            g.store_rating = *store_rating;
        }
    }

    // Append wishlist games (best-effort; needs a public wishlist).
    let owned: std::collections::HashSet<u32> = games.iter().map(|g| g.appid).collect();
    games.extend(fetch_wishlist_games(&client, steam_id, &owned).await);

    Ok(games)
}
