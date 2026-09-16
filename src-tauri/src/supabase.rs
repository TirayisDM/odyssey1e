//! Supabase transport — auth and PostgREST.
//!
//! WHY THIS LIVES IN RUST AND NOT THE WEBVIEW
//!   The data layer belongs in the engine. Keeping auth, session state
//!   and every query on this side means the frontend is a view and
//!   nothing more, and it leaves room for a local SQLite backend to sit
//!   behind the same function signatures later. It also matches how
//!   odyssey-engine already talks to PostgREST.
//!
//! WHY BLOCKING AND NOT ASYNC
//!   Tauri runs a non-async #[tauri::command] on its own thread pool, so
//!   a blocking call here does not stall the UI or the tokio runtime.
//!   Async would buy nothing and cost a layer of complexity.
//!
//! WHY THE ANON KEY IS IN THE SOURCE
//!   It is a publishable key. It identifies the project, it does not
//!   grant anything, and it is designed to ship inside clients. The
//!   security boundary is RLS — see supabase/migrations/001. If a
//!   SERVICE ROLE key ever appears in this file, that is a bug: it
//!   bypasses RLS entirely and must never reach a client.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{Mutex, OnceLock};

pub const SUPABASE_URL: &str = "https://shraejtytdxmoxmxkwxq.supabase.co";
pub const SUPABASE_ANON_KEY: &str = "sb_publishable_Zx4Am4Yjt61lE7KPJUzVEA_IG4_Yfde";

/// One signed-in user. Held in memory only — a restart signs you out.
/// Persisting it is a later job and wants the OS keychain, not a file.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub access_token: String,
    pub refresh_token: String,
    pub user_id: String,
    pub email: String,
}

#[derive(Default)]
pub struct AppState {
    session: Mutex<Option<Session>>,
}

impl AppState {
    pub fn set(&self, s: Option<Session>) -> Result<(), String> {
        let mut guard = self
            .session
            .lock()
            .map_err(|_| "session lock poisoned".to_string())?;
        *guard = s;
        Ok(())
    }

    pub fn current(&self) -> Result<Option<Session>, String> {
        let guard = self
            .session
            .lock()
            .map_err(|_| "session lock poisoned".to_string())?;
        Ok(guard.clone())
    }

    /// The access token, or a readable error. Every data call goes
    /// through here, so "not signed in" is stated once rather than
    /// rediscovered as a 401 at each call site.
    pub fn token(&self) -> Result<String, String> {
        match self.current()? {
            Some(s) => Ok(s.access_token),
            None => Err("not signed in".to_string()),
        }
    }
}

fn http() -> &'static reqwest::blocking::Client {
    static CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();
    CLIENT.get_or_init(reqwest::blocking::Client::new)
}

/// Pull something readable out of an error body. GoTrue and PostgREST
/// disagree about which key holds the message, so try the ones they
/// actually use before falling back to the raw text.
fn error_message(status: u16, body: &str) -> String {
    if let Ok(v) = serde_json::from_str::<Value>(body) {
        for key in ["error_description", "message", "msg", "error", "hint"] {
            if let Some(s) = v.get(key).and_then(|x| x.as_str()) {
                if !s.is_empty() {
                    return format!("{} ({})", s, status);
                }
            }
        }
    }
    if body.is_empty() {
        format!("HTTP {}", status)
    } else {
        format!("HTTP {}: {}", status, body)
    }
}

/* ============================ AUTH ============================ */

#[derive(Deserialize)]
struct AuthUser {
    id: String,
    email: Option<String>,
}

#[derive(Deserialize)]
struct AuthResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    user: Option<AuthUser>,
}

fn auth_call(path: &str, body: &Value) -> Result<Session, String> {
    let resp = http()
        .post(format!("{}/auth/v1/{}", SUPABASE_URL, path))
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .map_err(|e| format!("could not reach Supabase: {}", e))?;

    let status = resp.status().as_u16();
    let text = resp.text().unwrap_or_default();

    if !(200..300).contains(&status) {
        return Err(error_message(status, &text));
    }

    let parsed: AuthResponse =
        serde_json::from_str(&text).map_err(|e| format!("unexpected auth response: {}", e))?;

    // Signup with email confirmation ON returns a user and no session.
    // That is not an error, but it is not a signed-in state either —
    // say so plainly rather than handing back an empty token.
    let access_token = parsed.access_token.ok_or_else(|| {
        "signed up, but no session was returned — email confirmation is probably still on"
            .to_string()
    })?;
    let user = parsed
        .user
        .ok_or_else(|| "auth response had no user".to_string())?;

    Ok(Session {
        access_token,
        refresh_token: parsed.refresh_token.unwrap_or_default(),
        user_id: user.id,
        email: user.email.unwrap_or_default(),
    })
}

pub fn sign_up(email: &str, password: &str, display_name: &str) -> Result<Session, String> {
    // display_name rides in as user metadata; the handle_new_user()
    // trigger in migration 001 reads it to seed public.profiles.
    auth_call(
        "signup",
        &json!({
            "email": email,
            "password": password,
            "data": { "display_name": display_name }
        }),
    )
}

pub fn sign_in(email: &str, password: &str) -> Result<Session, String> {
    auth_call(
        "token?grant_type=password",
        &json!({ "email": email, "password": password }),
    )
}

/* ============================ POSTGREST ============================ */

fn rest_url(path: &str) -> String {
    format!("{}/rest/v1/{}", SUPABASE_URL, path)
}

/// GET against a table or view. `query` is passed through as PostgREST
/// filter syntax, e.g. [("select", "*"), ("game_id", "eq.<uuid>")].
pub fn rest_get(token: &str, path: &str, query: &[(&str, &str)]) -> Result<Value, String> {
    let resp = http()
        .get(rest_url(path))
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Authorization", format!("Bearer {}", token))
        .query(query)
        .send()
        .map_err(|e| format!("could not reach Supabase: {}", e))?;

    let status = resp.status().as_u16();
    let text = resp.text().unwrap_or_default();
    if !(200..300).contains(&status) {
        return Err(error_message(status, &text));
    }
    serde_json::from_str(&text).map_err(|e| format!("bad JSON from Supabase: {}", e))
}

/// INSERT. Prefer: return=representation so the caller gets the row
/// back — including every column a trigger or default filled in, which
/// is most of what makes migration 001 worth having.
pub fn rest_insert(token: &str, path: &str, body: &Value) -> Result<Value, String> {
    let resp = http()
        .post(rest_url(path))
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .header("Prefer", "return=representation")
        .json(body)
        .send()
        .map_err(|e| format!("could not reach Supabase: {}", e))?;

    let status = resp.status().as_u16();
    let text = resp.text().unwrap_or_default();
    if !(200..300).contains(&status) {
        return Err(error_message(status, &text));
    }
    serde_json::from_str(&text).map_err(|e| format!("bad JSON from Supabase: {}", e))
}

/// PATCH. `filter` narrows the rows, e.g. [("id", "eq.<uuid>")].
/// An empty filter would update every row RLS lets you touch, so it is
/// refused rather than trusted.
pub fn rest_update(
    token: &str,
    path: &str,
    filter: &[(&str, &str)],
    body: &Value,
) -> Result<Value, String> {
    if filter.is_empty() {
        return Err("refusing to update without a filter".to_string());
    }
    let resp = http()
        .patch(rest_url(path))
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .header("Prefer", "return=representation")
        .query(filter)
        .json(body)
        .send()
        .map_err(|e| format!("could not reach Supabase: {}", e))?;

    let status = resp.status().as_u16();
    let text = resp.text().unwrap_or_default();
    if !(200..300).contains(&status) {
        return Err(error_message(status, &text));
    }
    serde_json::from_str(&text).map_err(|e| format!("bad JSON from Supabase: {}", e))
}

/// Call a Postgres function. join_game() is the one that matters.
pub fn rpc(token: &str, func: &str, body: &Value) -> Result<Value, String> {
    let resp = http()
        .post(format!("{}/rest/v1/rpc/{}", SUPABASE_URL, func))
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .map_err(|e| format!("could not reach Supabase: {}", e))?;

    let status = resp.status().as_u16();
    let text = resp.text().unwrap_or_default();
    if !(200..300).contains(&status) {
        return Err(error_message(status, &text));
    }
    if text.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&text).map_err(|e| format!("bad JSON from Supabase: {}", e))
}
