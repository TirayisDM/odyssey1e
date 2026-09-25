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

use serde::{Deserialize, Deserializer, Serialize};
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

/* ============================ NUMERIC ============================ */

// ONE PLACE TO BE RIGHT ABOUT `numeric`, because being wrong about it in
// seven places cost three bugs and is still written down wrong in two.
//
// Postgres serialises `numeric` to a JSON NUMBER. `to_json(2.5::numeric)`
// is `2.5`, not `"2.5"` - verified against the live database rather than
// reasoned about, which is how this was settled the third time.
//
//   58fdc30  three readers took `as_str` alone, so every weight and
//            every container capacity in the game read as None. The
//            sheet said a character in scale mail weighed nothing.
//   3e03c4e  holders::Obj declared Option<String>, and `Obj` is
//            deserialised by serde, where the declared type IS the
//            parser. It refused the whole array rather than one field,
//            and the Objects tab went blank because one sword had been
//            given a weight.
//
// A LENIENT READER HIDES A WRONG BELIEF until somebody writes a strict
// one. containers::as_f64 took both representations and carried a
// comment claiming the wrong one, and the comment was copied into files
// where only that branch existed. So the leniency lives here now, once,
// next to the sentence saying which representation is actually real -
// and no caller gets to hold an opinion about it.

/// A `numeric` column as a number.
pub fn numeric(v: Option<&Value>) -> Option<f64> {
    let v = v?;
    v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

/// The same, by key off a row.
pub fn numeric_at(row: &Value, key: &str) -> Option<f64> {
    numeric(row.get(key))
}

/// A `numeric` column as text, for printing.
///
/// Separate because a weight is SHOWN far more often than it is summed,
/// and `f64::to_string` gives `5` for a whole number rather than `5.0`,
/// which is what a sheet should say.
pub fn numeric_text_at(row: &Value, key: &str) -> Option<String> {
    numeric_at(row, key).map(|n| n.to_string())
}

/// For a struct field serde deserialises directly.
///
/// THE STRUCT CASE IS THE DANGEROUS ONE. A hand-written reader that
/// guesses wrong loses one field; a derived one refuses the entire
/// response. This takes a number or a string either way, so declaring
/// the field wrong cannot repeat 3e03c4e.
///
/// ```text
/// #[serde(default, deserialize_with = "crate::supabase::de_numeric")]
/// pub weight_override: Option<f64>,
/// ```
pub fn de_numeric<'de, D>(d: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Option::<Value>::deserialize(d)?;
    Ok(numeric(v.as_ref()))
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

/// Trade a refresh token for a live session.
///
/// The same endpoint sign_in uses with a different grant, which is why
/// it returns the same shape and needs no parsing of its own. A refresh
/// token is single-use at Supabase: the response carries a NEW one, and
/// whoever stored the old one has to store the new one or the next
/// unlock fails. See commands/session.rs, which does exactly that.
pub fn refresh(refresh_token: &str) -> Result<Session, String> {
    auth_call(
        "token?grant_type=refresh_token",
        &json!({ "refresh_token": refresh_token }),
    )
}

/* ============================ POSTGREST ============================ */

fn rest_url(path: &str) -> String {
    format!("{}/rest/v1/{}", SUPABASE_URL, path)
}

/// GET against a table or view. `query` is passed through as PostgREST
/// filter syntax, e.g. [("select", "*"), ("game_id", "eq.<uuid>")].
pub fn rest_get(token: &str, path: &str, query: &[(&str, &str)]) -> Result<Value, String> {
    check_query(query)?;
    let query = tidy_select(query);
    let resp = http()
        .get(rest_url(path))
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Authorization", format!("Bearer {}", token))
        .query(&query)
        .send()
        .map_err(|e| format!("could not reach Supabase: {}", e))?;

    let status = resp.status().as_u16();
    let text = resp.text().unwrap_or_default();
    if !(200..300).contains(&status) {
        return Err(error_message(status, &text));
    }
    serde_json::from_str(&text).map_err(|e| format!("bad JSON from Supabase: {}", e))
}

/// A control character in a query parameter is always a typo.
///
/// THE BUG THIS EXISTS FOR. A `select` list was wrapped across two
/// lines with an escaped `\n` where it needed a Rust line continuation
/// - a lone backslash. The string then carried a real newline and
/// twenty-four spaces INTO the query, so PostgREST was asked for a
/// column named "<newline><spaces>size_override" and refused the entire
/// request.
///
/// It compiled, it passed every test, and it was committed, because
/// nothing here runs against a database. On screen the whole object
/// manager simply went empty - and an empty list renders as "no objects
/// yet", which is a sentence about the game rather than about a broken
/// query.
///
/// So the failure is moved forward to where it can name itself. No
/// legitimate filter or order contains a newline, a tab or a carriage
/// return; PostgREST's grammar has no use for one.
///
/// A SELECT IS THE EXCEPTION, and only for whitespace. Its whitespace
/// is always a wrapping accident and never changes what was asked for,
/// so `tidy_select` removes it instead - repairing the request rather
/// than refusing it, for the reason written there. Anything else
/// control-shaped in a select is still a typo and still refused.
fn check_query(query: &[(&str, &str)]) -> Result<(), String> {
    for (k, v) in query {
        let repairable = |c: &char| *k == "select" && c.is_whitespace();
        if let Some(bad) = v.chars().find(|c| c.is_control() && !repairable(c)) {
            return Err(format!(
                "the '{}' parameter contains {:?}, which is a typo rather than a filter \
                 - a wrapped string needs a line continuation, not an escape",
                k, bad
            ));
        }
    }
    Ok(())
}

/// The same typo with the newline already eaten — repaired rather than
/// refused.
///
/// Two wrapped lines joined into one leave a run of indentation in the
/// middle of a column list: "...,face_outcome,        action_id". It
/// holds no control character, so `check_query` never saw it, and it
/// was sitting in `list_rolls` AND in `load_profile`.
///
/// REFUSING IT WAS THE WRONG FIX, and it took about a minute to find
/// out: the guard shipped, `list_npc_attacks` went red, and every
/// goblin on the Run tab read "no weapon". PostgREST had been
/// tolerating those spaces all along. Turning a harmless typo into a
/// hard failure is not making the two paths different - it is a third
/// path, and the one the DM sees.
///
/// So it is normalised. A SELECT IS THE ONE PARAMETER WITH NO USE FOR
/// WHITESPACE - column names, commas, and the parentheses of an
/// embedded resource - so removing it cannot change what was asked
/// for, and the request becomes exactly what the author meant. It also
/// repairs 036's original case, where a real newline made PostgREST
/// refuse the lot and every object vanished.
///
/// EVERY OTHER PARAMETER IS LEFT ALONE. `label=eq.Goblin Scout` is an
/// ordinary filter on a name with a space in it, and stripping that
/// would break searching by name.
fn tidy_select<'a>(query: &[(&'a str, &'a str)]) -> Vec<(&'a str, String)> {
    query
        .iter()
        .map(|(k, v)| {
            if *k == "select" && v.chars().any(|c| c.is_whitespace()) {
                (*k, v.chars().filter(|c| !c.is_whitespace()).collect())
            } else {
                (*k, v.to_string())
            }
        })
        .collect()
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

/// DELETE. Same refusal as rest_update: an unfiltered delete would take
/// every row RLS allows, which is never what anyone meant.
pub fn rest_delete(token: &str, path: &str, filter: &[(&str, &str)]) -> Result<(), String> {
    if filter.is_empty() {
        return Err("refusing to delete without a filter".to_string());
    }
    let resp = http()
        .delete(rest_url(path))
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Authorization", format!("Bearer {}", token))
        .query(filter)
        .send()
        .map_err(|e| format!("could not reach Supabase: {}", e))?;

    let status = resp.status().as_u16();
    let text = resp.text().unwrap_or_default();
    if !(200..300).contains(&status) {
        return Err(error_message(status, &text));
    }
    Ok(())
}

/// INSERT ... ON CONFLICT DO UPDATE. `on_conflict` names the constraint
/// columns, comma separated — PostgREST needs them spelled out because
/// it cannot infer which unique index you meant.
pub fn rest_upsert(
    token: &str,
    path: &str,
    body: &Value,
    on_conflict: &str,
) -> Result<Value, String> {
    let resp = http()
        .post(rest_url(path))
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .header("Prefer", "return=representation,resolution=merge-duplicates")
        .query(&[("on_conflict", on_conflict)])
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


/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_ordinary_query_passes() {
        assert!(check_query(&[
            ("select", "id,item_key,name,quantity,equipped,holder_id,entity_id"),
            ("game_id", "eq.11111111-2222-3333-4444-555555555555"),
            ("order", "item_key.asc,acquired_at.asc"),
            ("or", "(game_id.is.null,game_id.eq.abc)"),
        ])
        .is_ok());
    }

    // THE ONE THAT GOT THROUGH, 036's. An escaped \n where a line
    // continuation was meant, which compiles, tests clean, and empties
    // a screen. It is now REPAIRED rather than refused: the request
    // that goes out is the one the author meant.
    #[test]
    fn a_wrapped_select_with_a_real_newline_is_repaired() {
        let q = [(
            "select",
            "id,item_key,holder_id,entity_id,\n                        size_override",
        )];
        assert!(check_query(&q).is_ok());
        assert_eq!(
            tidy_select(&q)[0].1,
            "id,item_key,holder_id,entity_id,size_override"
        );
    }

    // Outside a select, a control character has nothing to repair it
    // into - an order or a filter can hold a real value with a space,
    // so guessing at what was meant would be guessing.
    #[test]
    fn tabs_and_returns_are_still_refused_elsewhere() {
        assert!(check_query(&[("order", "name.asc\r")]).is_err());
        assert!(check_query(&[("name", "eq.Goblin\tScout")]).is_err());
    }

    // A SPACE IS NOT A CONTROL CHARACTER and must stay legal: a name
    // filter is a perfectly good place for one.
    #[test]
    fn a_space_is_allowed_because_names_have_them() {
        assert!(check_query(&[("name", "eq.Rodnar Shieldcrest")]).is_ok());
    }

    // THE SECOND ONE THAT GOT THROUGH, and it got through the fix for
    // the first. Same wrapped-string mistake with the newline already
    // removed, leaving only indentation — no control character, so the
    // check had nothing to catch. This is the exact string that was in
    // `list_rolls`, and `load_profile` carried one just like it.
    #[test]
    fn a_select_carrying_indentation_is_repaired() {
        let q = [(
            "select",
            "id,created_at,total,margin,face_outcome,                 action_id,role",
        )];
        assert_eq!(
            tidy_select(&q)[0].1,
            "id,created_at,total,margin,face_outcome,action_id,role"
        );
    }

    // EVERY OTHER PARAMETER IS LEFT EXACTLY AS IT WAS. Stripping a
    // filter's spaces would turn a search for a named goblin into a
    // search for a different string - this is why tidying is scoped to
    // the one parameter that can never want whitespace.
    #[test]
    fn a_filters_spaces_survive_the_tidy() {
        let q = [("select", "id,name"), ("label", "eq.Goblin Scout")];
        let out = tidy_select(&q);
        assert_eq!(out[1].1, "eq.Goblin Scout");
    }

    // The select the encounter log actually sends: an embedded resource
    // in parentheses, which must pass untouched.
    #[test]
    fn an_embedded_resource_is_still_an_ordinary_select() {
        let q = [("select", "id,round,key,rolls(id,role,total,success,margin)")];
        assert!(check_query(&q).is_ok());
        assert_eq!(tidy_select(&q)[0].1, q[0].1);
    }

    /* ------------------------- numeric ------------------------- */

    #[test]
    fn numeric_takes_the_json_number_that_actually_arrives() {
        // to_json(2.5::numeric) is 2.5. This is the real case, and the
        // one three hand-written readers used to miss.
        let row = json!({ "weight": 45, "slots": 0.25, "capacity_slots": 2.5 });
        assert_eq!(numeric_at(&row, "weight"), Some(45.0));
        assert_eq!(numeric_at(&row, "slots"), Some(0.25));
        assert_eq!(numeric_at(&row, "capacity_slots"), Some(2.5));
    }

    #[test]
    fn and_a_string_too_because_this_repo_believed_that_for_weeks() {
        let row = json!({ "weight": "45", "slots": "0.25" });
        assert_eq!(numeric_at(&row, "weight"), Some(45.0));
        assert_eq!(numeric_at(&row, "slots"), Some(0.25));
    }

    #[test]
    fn a_missing_column_is_none_rather_than_zero() {
        // None is "the row did not say". Zero would be weightless, and a
        // sack of weightless things is how encumbrance broke.
        let row = json!({ "key": "greatsword" });
        assert_eq!(numeric_at(&row, "weight"), None);
        assert_eq!(numeric_text_at(&row, "weight"), None);
    }

    #[test]
    fn nonsense_is_none_and_does_not_panic() {
        let row = json!({ "weight": "heavy", "slots": true, "price": null });
        assert_eq!(numeric_at(&row, "weight"), None);
        assert_eq!(numeric_at(&row, "slots"), None);
        assert_eq!(numeric_at(&row, "price"), None);
    }

    #[test]
    fn text_prints_a_whole_number_without_its_decimal() {
        // 5 rather than 5.0, which is what belongs on a sheet.
        let row = json!({ "weight": 5, "slots": 0.5 });
        assert_eq!(numeric_text_at(&row, "weight").as_deref(), Some("5"));
        assert_eq!(numeric_text_at(&row, "slots").as_deref(), Some("0.5"));
    }

    #[derive(Deserialize)]
    struct NumRow {
        #[serde(default, deserialize_with = "de_numeric")]
        weight_override: Option<f64>,
    }

    // 3e03c4e: a String field here refused the WHOLE array, and the
    // Objects tab showed nothing because one sword had a weight.
    #[test]
    fn the_struct_case_takes_a_number_a_string_or_nothing() {
        let cases = [
            (json!({ "weight_override": 5.0 }), Some(5.0)),
            (json!({ "weight_override": 5 }), Some(5.0)),
            (json!({ "weight_override": "5" }), Some(5.0)),
            (json!({ "weight_override": 0.25 }), Some(0.25)),
            (json!({ "weight_override": null }), None),
            (json!({}), None),
        ];
        for (row, want) in cases {
            let got: NumRow = serde_json::from_value(row.clone())
                .unwrap_or_else(|e| panic!("refused {}: {}", row, e));
            assert_eq!(got.weight_override, want, "for {}", row);
        }
    }
}
