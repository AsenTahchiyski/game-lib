use serde::{Deserialize, Serialize};

// IGN's Playlist library is served from an undocumented Apollo GraphQL backend.
// Public profiles are readable with no auth — the website itself fetches the
// same way client-side. We reproduce the two operations the playlist page uses:
//   PlaylistUser(nickname) -> resolve the handle to a userId (+ privacy)
//   SearchLibrary(userId)  -> the games, paginated by cursor, with status flags
//
// Unlike Steam/GOG/Epic (which convey *ownership* only), IGN is the source of
// curated *statuses* — this is the one-time migration the user actually wants.
const GRAPHQL_URL: &str = "https://mollusk.apis.ign.com/graphql";

const PLAYLIST_USER_QUERY: &str = r#"
query PlaylistUser($nickname: String) {
  playlistUser(nickname: $nickname) {
    id
    nickname
    playlistSettings { privacy }
  }
}"#;

// Note: the default (unfiltered) searchLibrary returns only "in-library" items
// and EXCLUDES wishlist — wishlist is a separate bucket. We page it explicitly
// with $wishlist so those games come through too.
const SEARCH_LIBRARY_QUERY: &str = r#"
query SearchLibrary($userId: ID, $count: Int, $cursor: Cursor, $wishlist: Boolean) {
  searchLibrary(userId: $userId, count: $count, cursor: $cursor, wishlist: $wishlist, sortBy: "updatedAt", sortOrder: "desc") {
    pageInfo { hasNext nextCursor total }
    libraryObjects {
      objectId
      playing
      completed
      archived
      paused
      wishlist
      backlog
      object { metadata { names { name } } primaryImage { url } }
    }
  }
}"#;

#[derive(Deserialize)]
struct GraphQlResponse<T> {
    data: Option<T>,
    #[serde(default)]
    errors: Vec<GraphQlError>,
}

#[derive(Deserialize)]
struct GraphQlError {
    message: String,
}

#[derive(Deserialize)]
struct PlaylistUserData {
    #[serde(rename = "playlistUser")]
    playlist_user: Option<PlaylistUser>,
}

#[derive(Deserialize)]
struct PlaylistUser {
    id: String,
    #[serde(rename = "playlistSettings")]
    playlist_settings: PlaylistSettings,
}

#[derive(Deserialize)]
struct PlaylistSettings {
    privacy: String,
}

#[derive(Deserialize)]
struct SearchLibraryData {
    #[serde(rename = "searchLibrary")]
    search_library: SearchLibrary,
}

#[derive(Deserialize)]
struct SearchLibrary {
    #[serde(rename = "pageInfo")]
    page_info: PageInfo,
    #[serde(rename = "libraryObjects")]
    library_objects: Vec<LibraryObject>,
}

#[derive(Deserialize)]
struct PageInfo {
    #[serde(rename = "hasNext")]
    has_next: bool,
    #[serde(rename = "nextCursor")]
    next_cursor: Option<String>,
}

#[derive(Deserialize)]
struct LibraryObject {
    #[serde(rename = "objectId")]
    object_id: String,
    playing: bool,
    completed: bool,
    archived: bool,
    paused: bool,
    wishlist: bool,
    backlog: bool,
    object: Option<LibraryObjectInner>,
}

#[derive(Deserialize)]
struct LibraryObjectInner {
    metadata: Option<ObjectMetadata>,
    #[serde(rename = "primaryImage")]
    primary_image: Option<PrimaryImage>,
}

#[derive(Deserialize)]
struct PrimaryImage {
    url: Option<String>,
}

#[derive(Deserialize)]
struct ObjectMetadata {
    names: Option<ObjectNames>,
}

#[derive(Deserialize)]
struct ObjectNames {
    name: Option<String>,
}

/// One imported game with its status already mapped to our vocabulary.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IgnGame {
    pub id: String,
    pub title: String,
    pub status: String,
    pub cover_url: Option<String>,
}

/// Map IGN's six status booleans to one of our statuses. IGN has no "free"
/// status; its "archived" (dropped/shelved) maps to our "quit". If several are
/// set, pick by intent priority; if none, default to "backlog".
fn map_status(o: &LibraryObject) -> &'static str {
    if o.playing {
        "playing"
    } else if o.completed {
        "beat"
    } else if o.paused {
        "paused"
    } else if o.wishlist {
        "wishlist"
    } else if o.archived {
        "quit"
    } else if o.backlog {
        "backlog"
    } else {
        "backlog"
    }
}

/// Accept either a bare nickname or a full profile URL like
/// `https://www.ign.com/playlist/malkstor` and return the nickname.
fn parse_nickname(input: &str) -> String {
    let input = input.trim();
    if let Some(idx) = input.find("/playlist/") {
        let rest = &input[idx + "/playlist/".len()..];
        // strip any trailing path (/lists/...) or query string
        let end = rest.find(['/', '?']).unwrap_or(rest.len());
        rest[..end].trim().to_string()
    } else {
        input.to_string()
    }
}

async fn graphql<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    query: &str,
    variables: serde_json::Value,
) -> Result<T, String> {
    let resp = client
        .post(GRAPHQL_URL)
        .header("Content-Type", "application/json")
        .header("Origin", "https://www.ign.com")
        .header("Referer", "https://www.ign.com/")
        // Force an uncompressed response: the bundled HTTP client may advertise
        // gzip/brotli, and a compressed body it doesn't transparently decode
        // shows up as an opaque "error decoding response body".
        .header("Accept-Encoding", "identity")
        .json(&serde_json::json!({ "query": query, "variables": variables }))
        .send()
        .await
        .map_err(|e| format!("IGN request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("IGN API returned HTTP {}.", resp.status()));
    }

    // Read as text first, then parse, so a non-JSON body (e.g. a CDN
    // interstitial) is surfaced instead of a generic decode error.
    let text = resp
        .text()
        .await
        .map_err(|e| format!("Could not read IGN response: {e}"))?;

    let body: GraphQlResponse<T> = serde_json::from_str(&text).map_err(|e| {
        let snippet: String = text.chars().take(200).collect();
        format!("Could not parse IGN response: {e}. Body began: {snippet}")
    })?;

    if let Some(err) = body.errors.into_iter().next() {
        return Err(format!("IGN API error: {}", err.message));
    }
    body.data
        .ok_or_else(|| "IGN API returned no data.".to_string())
}

/// Import a public IGN Playlist library by nickname or profile URL. Resolves the
/// handle to a userId, verifies the playlist is public, then pages through the
/// whole library mapping each game's status into our vocabulary.
#[tauri::command]
pub async fn ign_sync(nickname: String) -> Result<Vec<IgnGame>, String> {
    let nickname = parse_nickname(&nickname);
    if nickname.is_empty() {
        return Err("Enter your IGN nickname or playlist profile URL.".into());
    }

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (game-lib)")
        .build()
        .map_err(|e| e.to_string())?;

    let user: PlaylistUserData = graphql(
        &client,
        PLAYLIST_USER_QUERY,
        serde_json::json!({ "nickname": nickname }),
    )
    .await?;

    let user = user
        .playlist_user
        .ok_or_else(|| format!("No IGN user found for \"{nickname}\"."))?;

    if !user.playlist_settings.privacy.eq_ignore_ascii_case("public") {
        return Err(
            "This IGN playlist is not public. Set its privacy to Public in IGN, then retry."
                .into(),
        );
    }

    // Two buckets: the default in-library set, and wishlist (which the default
    // query omits). `wishlist: None` is the unfiltered in-library set.
    let mut objects = fetch_bucket(&client, &user.id, None).await?;
    objects.extend(fetch_bucket(&client, &user.id, Some(true)).await?);

    Ok(objects
        .iter()
        .map(|o| {
            let title = o
                .object
                .as_ref()
                .and_then(|i| i.metadata.as_ref())
                .and_then(|m| m.names.as_ref())
                .and_then(|n| n.name.clone())
                .unwrap_or_else(|| format!("IGN game {}", o.object_id));
            let cover_url = o
                .object
                .as_ref()
                .and_then(|i| i.primary_image.as_ref())
                .and_then(|p| p.url.clone());
            IgnGame {
                id: o.object_id.clone(),
                title,
                status: map_status(o).to_string(),
                cover_url,
            }
        })
        .collect())
}

/// Page through one searchLibrary bucket. `wishlist = Some(true)` fetches the
/// wishlist bucket; `None` fetches the default in-library set (which excludes
/// wishlist). Follows the cursor until the last page.
async fn fetch_bucket(
    client: &reqwest::Client,
    user_id: &str,
    wishlist: Option<bool>,
) -> Result<Vec<LibraryObject>, String> {
    let mut objects = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let data: SearchLibraryData = graphql(
            client,
            SEARCH_LIBRARY_QUERY,
            serde_json::json!({
                "userId": user_id,
                "count": 100,
                "cursor": cursor,
                "wishlist": wishlist,
            }),
        )
        .await?;

        let page = data.search_library;
        objects.extend(page.library_objects);

        match (page.page_info.has_next, page.page_info.next_cursor) {
            (true, Some(c)) => cursor = Some(c),
            _ => break,
        }
    }
    Ok(objects)
}
