use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

// Epic Games Launcher's public OAuth client (same credentials used by legendary
// and Heroic). Not user secrets — they identify the launcher app.
const CLIENT_ID: &str = "34a02cf8f4414e29b15921876da36f9a";
const CLIENT_SECRET: &str = "daafbccc737745039dffe53d94fc76cf";

const TOKEN_URL: &str =
    "https://account-public-service-prod.ol.epicgames.com/account/api/oauth/token";
const ASSETS_URL: &str =
    "https://launcher-public-service-prod06.ol.epicgames.com/launcher/api/public/assets/Windows?label=Live";

// The real Epic launcher's User-Agent. The assets host sits behind Cloudflare
// (unlike the account host), which is unfriendly to clients sending none.
const USER_AGENT: &str =
    "UELauncher/11.0.1-14907503+++Portal+Release-Live Windows/10.0.19041.1.256.64bit";

/// Flatten an error's source chain. reqwest's own Display stops at "error
/// sending request for url (…)" and drops the part that says *why* — DNS
/// failure, TLS reset, timeout — which is the only useful half.
fn cause(e: &dyn std::error::Error) -> String {
    let mut msg = e.to_string();
    let mut source = e.source();
    while let Some(e) = source {
        msg.push_str(": ");
        msg.push_str(&e.to_string());
        source = e.source();
    }
    msg
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Could not build the Epic HTTP client: {}", cause(&e)))
}

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
    #[serde(default, rename = "keyImages")]
    key_images: Vec<KeyImage>,
}

#[derive(Deserialize)]
struct Category {
    #[serde(default)]
    path: String,
}

#[derive(Deserialize)]
struct KeyImage {
    #[serde(default, rename = "type")]
    image_type: String,
    #[serde(default)]
    url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EpicGame {
    pub id: String,
    pub title: String,
    pub cover_url: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EpicSyncResult {
    pub refresh_token: String,
    pub games: Vec<EpicGame>,
}

/// Choose the best box-art image: prefer a tall/portrait image, then any other
/// non-logo image. Returns `None` if nothing usable is present.
fn pick_cover(images: &[KeyImage]) -> Option<String> {
    const PREFERRED: [&str; 3] = ["DieselGameBoxTall", "OfferImageTall", "Thumbnail"];
    for ty in PREFERRED {
        if let Some(img) = images.iter().find(|i| i.image_type == ty && !i.url.is_empty()) {
            return Some(img.url.clone());
        }
    }
    images
        .iter()
        .find(|i| !i.url.is_empty() && i.image_type != "DieselGameBoxLogo")
        .map(|i| i.url.clone())
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
    let resp = client()?
        .post(TOKEN_URL)
        .basic_auth(CLIENT_ID, Some(CLIENT_SECRET))
        .form(form)
        .send()
        .await
        .map_err(|e| format!("Epic token request failed: {}", cause(&e)))?;
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

// Fake host the injected script navigates to in order to hand the code back.
// The navigation is always cancelled, so it never resolves.
const CALLBACK_HOST: &str = "gamelib.invalid";

/// Script injected into every page of the Epic login window.
///
/// Epic never puts the authorization code in a URL — it's only ever the body of
/// an API response. So once login has set a session cookie, poll that endpoint
/// same-origin (cookies included) and smuggle the code out through a navigation
/// to `CALLBACK_HOST`. Polling rather than reacting to the redirect keeps this
/// working on Android, where the JSON page may be downloaded instead of shown.
fn login_script() -> String {
    format!(
        r#"(function () {{
  if (!/(^|\.)epicgames\.com$/.test(location.hostname)) return;
  setInterval(function () {{
    fetch("/id/api/redirect?clientId={CLIENT_ID}&responseType=code", {{ credentials: "include" }})
      .then(function (r) {{ return r.json(); }})
      .then(function (d) {{
        if (d && d.authorizationCode) {{
          location.href = "https://{CALLBACK_HOST}/?code=" + encodeURIComponent(d.authorizationCode);
        }}
      }})
      .catch(function () {{}});
  }}, 2000);
}})();"#
    )
}

/// Log in through an embedded window and return the refresh token.
#[tauri::command]
pub async fn epic_login(app: tauri::AppHandle) -> Result<String, String> {
    let code = crate::login::capture_code(
        &app,
        "epic-login",
        "Log in to Epic",
        &epic_login_url(),
        &login_script(),
        |url| {
            if url.host_str() == Some(CALLBACK_HOST) {
                return match crate::login::query_param(url, "code") {
                    Some(code) => crate::login::Nav::Code(code),
                    None => crate::login::Nav::Allow,
                };
            }
            // Epic's own post-login redirect serves raw JSON; letting it load
            // would trigger a download prompt on Android. The poll above has
            // the code covered, so keep it from ever rendering.
            if url.path().starts_with("/id/api/redirect") {
                return crate::login::Nav::Block;
            }
            crate::login::Nav::Allow
        },
    )
    .await?;
    epic_exchange_code(code).await
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

    let client = client()?;

    // 1. Owned assets (every namespace/item the account owns).
    let assets: Vec<Asset> = client
        .get(ASSETS_URL)
        .bearer_auth(&token.access_token)
        .send()
        .await
        .map_err(|e| format!("Epic assets request failed: {}", cause(&e)))?
        .json()
        .await
        .map_err(|e| format!("Could not parse Epic assets: {}", cause(&e)))?;

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
            .map_err(|e| format!("Epic catalog request failed: {}", cause(&e)))?;
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
                let cover_url = pick_cover(&item.key_images);
                games.push(EpicGame {
                    id,
                    title: item.title,
                    cover_url,
                });
            }
        }
    }

    Ok(EpicSyncResult {
        refresh_token: token.refresh_token,
        games,
    })
}
