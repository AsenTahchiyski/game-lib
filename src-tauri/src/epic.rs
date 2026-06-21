use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// Epic Games Launcher's public OAuth client (same credentials used by legendary
// and Heroic). Not user secrets — they identify the launcher app.
const CLIENT_ID: &str = "34a02cf8f4414e29b15921876da36f9a";
const CLIENT_SECRET: &str = "daafbccc737745039dffe53d94fc76cf";

const TOKEN_URL: &str =
    "https://account-public-service-prod.ol.epicgames.com/account/api/oauth/token";
const ASSETS_URL: &str =
    "https://launcher-public-service-prod06.ol.epicgames.com/launcher/api/public/assets/Windows?label=Live";

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Asset {
    catalog_item_id: String,
    namespace: String,
}

#[derive(Deserialize)]
struct CatalogItem {
    title: String,
    #[serde(default)]
    categories: Vec<Category>,
}

#[derive(Deserialize)]
struct Category {
    #[serde(default)]
    path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EpicGame {
    pub id: String,
    pub title: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EpicSyncResult {
    pub refresh_token: String,
    pub games: Vec<EpicGame>,
}

/// URL the user opens to log into Epic. After login Epic redirects to a page
/// that shows JSON containing an `authorizationCode` — the user pastes that.
#[tauri::command]
pub fn epic_login_url() -> String {
    format!(
        "https://www.epicgames.com/id/login?redirectUrl=\
         https%3A%2F%2Fwww.epicgames.com%2Fid%2Fapi%2Fredirect%3FclientId%3D{CLIENT_ID}%26responseType%3Dcode"
    )
}

async fn fetch_token(form: &[(&str, &str)]) -> Result<TokenResponse, String> {
    let resp = reqwest::Client::new()
        .post(TOKEN_URL)
        .basic_auth(CLIENT_ID, Some(CLIENT_SECRET))
        .form(form)
        .send()
        .await
        .map_err(|e| format!("Epic token request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "Epic token request returned HTTP {} — the code may be wrong or expired.",
            resp.status()
        ));
    }
    resp.json()
        .await
        .map_err(|e| format!("Could not parse Epic token response: {e}"))
}

/// Exchange the pasted authorization code for tokens; returns the refresh token.
#[tauri::command]
pub async fn epic_exchange_code(code: String) -> Result<String, String> {
    let token = fetch_token(&[
        ("grant_type", "authorization_code"),
        ("code", code.trim()),
        ("token_type", "eg1"),
    ])
    .await?;
    Ok(token.refresh_token)
}

/// Refresh the access token, list owned assets, then resolve titles via the
/// catalog service, keeping only items categorised as games.
#[tauri::command]
pub async fn epic_sync(refresh_token: String) -> Result<EpicSyncResult, String> {
    let token = fetch_token(&[
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token.trim()),
        ("token_type", "eg1"),
    ])
    .await?;

    let client = reqwest::Client::new();

    // 1. Owned assets (every namespace/item the account owns).
    let assets: Vec<Asset> = client
        .get(ASSETS_URL)
        .bearer_auth(&token.access_token)
        .send()
        .await
        .map_err(|e| format!("Epic assets request failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Could not parse Epic assets: {e}"))?;

    // 2. Group catalog item ids by namespace, skipping Unreal Engine content.
    let mut by_namespace: HashMap<String, Vec<String>> = HashMap::new();
    for asset in assets {
        if asset.namespace == "ue" {
            continue;
        }
        by_namespace
            .entry(asset.namespace)
            .or_default()
            .push(asset.catalog_item_id);
    }

    // 3. Resolve titles per namespace via the catalog bulk endpoint.
    let mut games = Vec::new();
    for (namespace, ids) in by_namespace {
        let url = format!(
            "https://catalog-public-service-prod06.ol.epicgames.com/catalog/api/shared/namespace/{namespace}/bulk/items"
        );
        let mut query: Vec<(&str, &str)> = ids.iter().map(|id| ("id", id.as_str())).collect();
        query.push(("includeMainGameDetails", "true"));
        query.push(("country", "US"));
        query.push(("locale", "en-US"));

        let resp = client
            .get(&url)
            .query(&query)
            .bearer_auth(&token.access_token)
            .send()
            .await
            .map_err(|e| format!("Epic catalog request failed: {e}"))?;
        if !resp.status().is_success() {
            // Skip a namespace we can't resolve rather than failing the whole sync.
            continue;
        }
        let items: HashMap<String, CatalogItem> = match resp.json().await {
            Ok(v) => v,
            Err(_) => continue,
        };

        for (id, item) in items {
            let is_game = item.categories.iter().any(|c| c.path == "games");
            if is_game {
                games.push(EpicGame {
                    id,
                    title: item.title,
                });
            }
        }
    }

    Ok(EpicSyncResult {
        refresh_token: token.refresh_token,
        games,
    })
}
