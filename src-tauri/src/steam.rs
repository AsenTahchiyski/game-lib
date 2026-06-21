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

    Ok(parsed
        .response
        .games
        .into_iter()
        .map(|g| SteamGame {
            appid: g.appid,
            name: g.name,
            playtime_minutes: g.playtime_forever,
        })
        .collect())
}
