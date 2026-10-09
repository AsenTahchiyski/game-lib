use serde::Serialize;

// Title search in one store's public catalog, for the Add-game dialog. All
// three endpoints are unauthenticated. Epic has no equivalent: its store
// search sits behind a Cloudflare challenge.

/// One search hit. `id` is the same id that store's sync uses, so an added
/// game later matches its synced copy by source id.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub store: String,
    pub id: String,
    pub title: String,
    pub cover_url: Option<String>, // thumbnail for the result list
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (game-lib)")
        .build()
        .map_err(|e| e.to_string())
}

async fn get_json(req: reqwest::RequestBuilder, what: &str) -> Result<serde_json::Value, String> {
    let resp = req
        .send()
        .await
        .map_err(|e| format!("{what} search failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("{what} search returned HTTP {}.", resp.status()));
    }
    resp.json()
        .await
        .map_err(|e| format!("Could not parse {what} search results: {e}"))
}

fn str_field(v: &serde_json::Value, key: &str) -> Option<String> {
    v.get(key).and_then(|s| s.as_str()).map(String::from)
}

async fn search_steam(term: &str) -> Result<Vec<SearchResult>, String> {
    let v = get_json(
        client()?
            .get("https://store.steampowered.com/api/storesearch/")
            .query(&[("term", term), ("l", "english"), ("cc", "US")]),
        "Steam",
    )
    .await?;
    let items = v.get("items").and_then(|i| i.as_array()).cloned().unwrap_or_default();
    Ok(items
        .iter()
        .filter_map(|it| {
            Some(SearchResult {
                store: "steam".into(),
                id: it.get("id")?.as_u64()?.to_string(),
                title: str_field(it, "name")?,
                cover_url: str_field(it, "tiny_image"),
            })
        })
        .collect())
}

async fn search_gog(term: &str) -> Result<Vec<SearchResult>, String> {
    let query = format!("like:{term}");
    let v = get_json(
        client()?.get("https://catalog.gog.com/v1/catalog").query(&[
            ("limit", "20"),
            ("query", query.as_str()),
            ("order", "desc:score"),
            ("productType", "in:game,pack"),
        ]),
        "GOG",
    )
    .await?;
    let products = v.get("products").and_then(|p| p.as_array()).cloned().unwrap_or_default();
    Ok(products
        .iter()
        .filter_map(|p| {
            // The id is a string here but a number in other GOG APIs.
            let id = match p.get("id")? {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                _ => return None,
            };
            Some(SearchResult {
                store: "gog".into(),
                id,
                title: str_field(p, "title")?,
                cover_url: str_field(p, "coverVertical"),
            })
        })
        .collect())
}

const IGN_SEARCH_QUERY: &str = r#"
query SearchObjects($name: String) {
  searchObjectsByName(name: $name, type: Game, count: 20) {
    objects { id metadata { names { name } } primaryImage { url } }
  }
}"#;

async fn search_ign(term: &str) -> Result<Vec<SearchResult>, String> {
    let v = get_json(
        client()?
            .post("https://mollusk.apis.ign.com/graphql")
            .header("Origin", "https://www.ign.com")
            .header("Referer", "https://www.ign.com/")
            // See ign.rs: avoid a compressed body the client won't decode.
            .header("Accept-Encoding", "identity")
            .json(&serde_json::json!({
                "query": IGN_SEARCH_QUERY,
                "variables": { "name": term },
            })),
        "IGN",
    )
    .await?;
    if let Some(msg) = v.pointer("/errors/0/message").and_then(|m| m.as_str()) {
        return Err(format!("IGN search error: {msg}"));
    }
    let objects = v
        .pointer("/data/searchObjectsByName/objects")
        .and_then(|o| o.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(objects
        .iter()
        .filter_map(|o| {
            Some(SearchResult {
                store: "ign".into(),
                id: str_field(o, "id")?,
                title: o.pointer("/metadata/names/name")?.as_str()?.to_string(),
                cover_url: o
                    .pointer("/primaryImage/url")
                    .and_then(|u| u.as_str())
                    .map(String::from),
            })
        })
        .collect())
}

/// Search one store's catalog by title.
#[tauri::command]
pub async fn store_search(store: String, term: String) -> Result<Vec<SearchResult>, String> {
    let term = term.trim();
    if term.is_empty() {
        return Ok(Vec::new());
    }
    match store.as_str() {
        "steam" => search_steam(term).await,
        "gog" => search_gog(term).await,
        "ign" => search_ign(term).await,
        _ => Err(format!("Search isn't available for {store}.")),
    }
}
