//! Embedded OAuth login windows, so connecting a store is "log in and click
//! confirm" instead of copying a code out of a browser by hand.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tokio::sync::oneshot;

/// How long to wait for the user to finish logging in. Without this a login
/// window that never reports being closed would leave the UI spinning forever.
const LOGIN_TIMEOUT: Duration = Duration::from_secs(300);

/// What to do with a navigation the login window is about to make.
pub enum Nav {
    Allow,
    /// Cancel the navigation without ending the login (used to stop a page we
    /// know we don't want rendered, or downloaded, from loading).
    Block,
    /// The OAuth code — ends the login.
    Code(String),
}

/// Open a login window at `url` and resolve with the OAuth code as soon as
/// `decide` recognises one in a navigation target. `script` is injected into
/// every page in the window; it's how a store that never puts the code in a URL
/// (Epic) gets to hand one over — by navigating to a URL that `decide` claims.
///
/// The window is created with a label of its own, so the app's capabilities
/// (scoped to `main`) don't apply: these pages get no access to Tauri APIs.
pub async fn capture_code(
    app: &AppHandle,
    label: &str,
    title: &str,
    url: &str,
    script: &str,
    decide: impl Fn(&Url) -> Nav + Send + 'static,
) -> Result<String, String> {
    let url: Url = url.parse().map_err(|e| format!("Bad login URL: {e}"))?;

    // Labels have to be unique, so clear out a window a previous attempt left
    // behind rather than failing the retry.
    if let Some(stale) = app.get_webview_window(label) {
        let _ = stale.destroy();
    }

    // Whichever happens first — a captured code or the window closing — wins;
    // taking the sender makes the other a no-op.
    let (tx, rx) = oneshot::channel::<Option<String>>();
    let tx = Arc::new(Mutex::new(Some(tx)));

    let nav_tx = tx.clone();
    let window = WebviewWindowBuilder::new(app, label, WebviewUrl::External(url))
        .title(title)
        .inner_size(520.0, 760.0)
        .initialization_script(script)
        .on_navigation(move |u| match decide(u) {
            Nav::Allow => true,
            Nav::Block => false,
            Nav::Code(code) => {
                if let Some(tx) = nav_tx.lock().unwrap().take() {
                    let _ = tx.send(Some(code));
                }
                false // the code was the whole point; don't load the callback
            }
        })
        .build()
        .map_err(|e| format!("Could not open the login window: {e}"))?;

    // Closing the window — or Back on Android, where it's a separate activity —
    // counts as cancelling.
    let close_tx = tx.clone();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Destroyed) {
            if let Some(tx) = close_tx.lock().unwrap().take() {
                let _ = tx.send(None);
            }
        }
    });

    let outcome = tokio::time::timeout(LOGIN_TIMEOUT, rx).await;
    let _ = window.close();
    match outcome {
        Ok(Ok(Some(code))) => Ok(code),
        Ok(_) => Err("Login was cancelled.".into()),
        Err(_) => Err("Timed out waiting for the login to finish.".into()),
    }
}

/// Value of a query parameter, if present.
pub fn query_param(url: &Url, name: &str) -> Option<String> {
    url.query_pairs()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.into_owned())
}
