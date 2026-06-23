use serde::{Deserialize, Serialize};

// GOG Galaxy's OAuth client. These are not secret — they're embedded in the
// public Galaxy client and used by every open-source GOG tool (gogdl, Heroic,
// Lutris). They identify the app, not the user.
const CLIENT_ID: &str = "46899977096215655";
const CLIENT_SECRET: &str = "9d85c43b1482497dbbce61f6e4aa173a433796eeae2ca8c5f6129f2dc4de46d9";
const REDIRECT_URI: &str = "https://embed.gog.com/on_login_success?origin=client";

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GogGame {
    pub id: String,
    pub title: String,
    pub cover_url: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GogSyncResult {
    pub refresh_token: String,
    pub games: Vec<GogGame>,
}

/// URL the user opens in a browser to log into GOG. After a successful login GOG
/// redirects to a page whose address contains `?code=...`; the user pastes that
/// code back into the app once.
#[tauri::command]
pub fn gog_login_url() -> String {
    format!(
        "https://auth.gog.com/auth?client_id={CLIENT_ID}\
         &redirect_uri=https%3A%2F%2Fembed.gog.com%2Fon_login_success%3Forigin%3Dclient\
         &response_type=code&layout=client2"
    )
}

async fn fetch_token(params: &[(&str, &str)]) -> Result<TokenResponse, String> {
    let resp = reqwest::Client::new()
        .get("https://auth.gog.com/token")
        .query(params)
        .send()
        .await
        .map_err(|e| format!("GOG token request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "GOG token request returned HTTP {} — the code may be wrong or expired.",
            resp.status()
        ));
    }
    resp.json()
        .await
        .map_err(|e| format!("Could not parse GOG token response: {e}"))
}

/// Exchange the pasted authorization code for tokens; returns the refresh token
/// to be stored for future one-click syncs.
#[tauri::command]
pub async fn gog_exchange_code(code: String) -> Result<String, String> {
    let token = fetch_token(&[
        ("client_id", CLIENT_ID),
        ("client_secret", CLIENT_SECRET),
        ("grant_type", "authorization_code"),
        ("code", code.trim()),
        ("redirect_uri", REDIRECT_URI),
    ])
    .await?;
    Ok(token.refresh_token)
}

#[derive(Deserialize)]
struct FilteredProducts {
    #[serde(rename = "totalPages")]
    total_pages: u32,
    products: Vec<GogProduct>,
}

#[derive(Deserialize)]
struct GogProduct {
    id: serde_json::Value, // GOG returns this as a number
    title: String,
    // Protocol-relative path without extension, e.g. "//images.gog.com/<hash>".
    #[serde(default)]
    image: Option<String>,
}

/// Refresh the access token and fetch all owned GOG games (id + title). Returns
/// the possibly-rotated refresh token plus the games.
#[tauri::command]
pub async fn gog_sync(refresh_token: String) -> Result<GogSyncResult, String> {
    let token = fetch_token(&[
        ("client_id", CLIENT_ID),
        ("client_secret", CLIENT_SECRET),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token.trim()),
    ])
    .await?;

    let client = reqwest::Client::new();
    let mut games = Vec::new();
    let mut page = 1u32;
    loop {
        let page_str = page.to_string();
        let resp = client
            .get("https://embed.gog.com/account/getFilteredProducts")
            .query(&[("mediaType", "1"), ("page", page_str.as_str())])
            .bearer_auth(&token.access_token)
            .send()
            .await
            .map_err(|e| format!("GOG library request failed: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("GOG library request returned HTTP {}.", resp.status()));
        }
        let data: FilteredProducts = resp
            .json()
            .await
            .map_err(|e| format!("Could not parse GOG library: {e}"))?;

        for p in data.products {
            let id = match p.id {
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::String(s) => s,
                _ => continue,
            };
            // GOG's `image` is a protocol-relative, extension-less path; the
            // _392 box-art variant is a reliable size to request.
            let cover_url = p
                .image
                .filter(|s| !s.is_empty())
                .map(|s| format!("https:{s}_392.jpg"));
            games.push(GogGame {
                id,
                title: p.title,
                cover_url,
            });
        }

        if data.total_pages == 0 || page >= data.total_pages {
            break;
        }
        page += 1;
    }

    Ok(GogSyncResult {
        refresh_token: token.refresh_token,
        games,
    })
}
