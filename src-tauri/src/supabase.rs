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
///
/// `user_id` AND `profile_id` ARE NOT THE SAME QUESTION (197).
/// `user_id` is the SIGN-IN — one row in `auth.users`, one way of
/// proving who you are, and replaceable. `profile_id` is the PERSON,
/// and it is what every ownership column in the schema holds:
/// `owner_uid`, `dm_uid`, `game_members.profile_id`.
///
/// They hold the same value today, because every account has exactly
/// one sign-in. They stop being equal the moment anyone links a second
/// one, and then writing `user_id` into an ownership column produces a
/// row owned by nobody. Both are kept so the code has to say which it
/// means.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub access_token: String,
    pub refresh_token: String,
    pub user_id: String,
    pub profile_id: String,
    pub email: String,
    /// 200. WHEN THE ACCESS TOKEN STOPS WORKING, as unix seconds.
    ///
    /// GoTrue has always sent this and we always threw it away, which
    /// is why nothing could renew: you cannot refresh before expiry
    /// without knowing when expiry is.
    ///
    /// ZERO MEANS UNKNOWN and is treated as "refresh now" - a session
    /// from an older build, or a response that omitted it. Guessing
    /// "probably fine" would reproduce the bug this closes.
    #[serde(default)]
    pub expires_at: i64,
}

/// How long before expiry to go and get a new token, in seconds.
///
/// NOT ZERO, for two reasons that both bite. The refresh itself takes a
/// round trip, and a token that was valid when the check ran can be
/// dead by the time the request lands. And the two clocks are not the
/// same clock - a device a minute fast would refresh a minute late,
/// every time.
///
/// AND NOT HUGE. Set larger than the token's own lifetime it forces a
/// refresh before EVERY database read, which is how the path was first
/// watched end to end - and which GoTrue answers with `429 Request rate
/// limit reached` within a few seconds. See 201.
pub const REFRESH_MARGIN_SECS: i64 = 120;

/// Whether this access token should be replaced before it is used.
///
/// 200. THE WHOLE RULE, kept here as one tested function rather than an
/// inline comparison, because "is it time yet" is the thing that will
/// be got wrong - off by a sign, or by the margin, or by trusting a
/// zero.
pub fn needs_refresh(expires_at: i64, now: i64) -> bool {
    expires_at <= 0 || now + REFRESH_MARGIN_SECS >= expires_at
}

/// Whether this access token would still be accepted right now.
///
/// 201. A DIFFERENT QUESTION FROM `needs_refresh`, and the distinction
/// is the whole point. Inside the margin both are true: the token wants
/// replacing AND it still works. Past expiry only the first is. The
/// margin is only worth having if something spends the life it
/// reserves, and this is what spends it.
///
/// A zero is UNKNOWN and therefore not usable - same reading as
/// `needs_refresh` gives it, because a token we cannot date is one we
/// cannot vouch for.
pub fn still_usable(expires_at: i64, now: i64) -> bool {
    expires_at > 0 && now < expires_at
}

/// Unix seconds, now.
pub fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[derive(Default)]
pub struct AppState {
    session: Mutex<Option<Session>>,
    /// 200. WHERE THE FAST-LOGIN FILE IS, when one is set up.
    ///
    /// THE SESSION IS THE THING BEING PERSISTED, so keeping the stored
    /// copy in step with the live one belongs here rather than in each
    /// caller. `pin.rs` owns the file's SHAPE and its hashing and stays
    /// pure; this owns the fact that a refresh token which has rotated
    /// must be written down again.
    ///
    /// WITHOUT THIS, AUTO-REFRESH WOULD BREAK THE PIN. A refresh token
    /// is single-use: spending one kills it and issues another. The
    /// file would be left holding the dead one, and the PIN would work
    /// exactly once - which is the bug `unlock` already had to fix in
    /// its own path and names in its comment.
    pin_path: Mutex<Option<std::path::PathBuf>>,
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
        // 200. AND IT RENEWS ITSELF HERE.
        //
        // An access token lasts about an hour and nothing ever replaced
        // it, so a long session died partway through and the only cure
        // was signing in again. `refresh` existed from the start and
        // `unlock` was its only caller.
        //
        // THIS IS THE RIGHT PLACE because it is already the one every
        // data call goes through - the same reason "not signed in" is
        // said here once instead of being rediscovered as a 401 at a
        // hundred call sites.
        //
        // THE LOCK IS HELD ACROSS THE NETWORK CALL, deliberately. Two
        // commands starting at once would both see an expiring token
        // and both refresh; the second would spend a token the first
        // had just invalidated, fail, and sign the user out. Holding
        // the lock makes the second one wait and then find the work
        // already done. It costs one blocked moment an hour, and
        // nothing here re-enters the lock, so it cannot deadlock.
        let mut guard = self
            .session
            .lock()
            .map_err(|_| "session lock poisoned".to_string())?;
        let Some(live) = guard.clone() else {
            return Err("not signed in".to_string());
        };
        // ONE READING OF THE CLOCK for the whole decision. Asking
        // twice would let the token be judged live by the first
        // question and dead by the second.
        let now = now_secs();
        if !needs_refresh(live.expires_at, now) {
            return Ok(live.access_token);
        }
        if live.refresh_token.is_empty() {
            // NOTHING TO SPEND. Say so plainly rather than letting the
            // stale token go out and come back as an unexplained 401.
            return Err("your session has expired - sign in again".to_string());
        }

        // A FAILED REFRESH LEAVES THE OLD SESSION IN PLACE and falls
        // back to the token it already has.
        //
        // 201. THIS WAS A FAULT WEARING THE COSTUME OF AN ORDINARY
        // ANSWER. The comment above said the old session was kept, and
        // it was - but the `?` on the end of this call still failed the
        // read the user asked for, with a token in hand that had up to
        // REFRESH_MARGIN_SECS of life left. The forced-margin test
        // showed it as a wall of `could not renew your session` on
        // calls that would all have worked.
        //
        // AND IT ONLY FALLS BACK WHILE THE OLD TOKEN IS REALLY ALIVE.
        // Past expiry there is nothing to fall back TO, and handing out
        // a dead token would trade this clear message for an
        // unexplained 401 at the call site - which is the thing the
        // whole accessor exists to prevent.
        let fresh = match refresh(&live.refresh_token) {
            Ok(fresh) => fresh,
            Err(_) if still_usable(live.expires_at, now) => return Ok(live.access_token),
            Err(e) => return Err(format!("could not renew your session: {}", e)),
        };
        let token = fresh.access_token.clone();
        self.reseat_pin(&fresh);
        *guard = Some(fresh);
        Ok(token)
    }

    /// Remember where the fast-login file is, so a rotated refresh
    /// token can be written back to it.
    pub fn set_pin_path(&self, path: Option<std::path::PathBuf>) {
        if let Ok(mut g) = self.pin_path.lock() {
            *g = path;
        }
    }

    /// Write the current refresh token into the fast-login file, if
    /// there is one.
    ///
    /// 200. BEST EFFORT, AND SILENT ON FAILURE. The session in memory
    /// is good either way; a file that could not be written means the
    /// PIN will be refused next time, which is recoverable by signing
    /// in. Failing the data call the user actually asked for, because a
    /// convenience file is read-only, would be the worse trade.
    ///
    /// NOTHING IS CREATED HERE. If no PIN has been set there is no file
    /// and none is made - a refresh must not quietly start storing a
    /// credential on disk that the user never asked it to store.
    fn reseat_pin(&self, s: &Session) {
        let Ok(guard) = self.pin_path.lock() else { return };
        let Some(path) = guard.clone() else { return };
        let Ok(text) = std::fs::read_to_string(&path) else { return };
        let Ok(mut stored) = serde_json::from_str::<crate::pin::Stored>(&text) else { return };
        if stored.refresh_token == s.refresh_token {
            return;
        }
        stored.refresh_token = s.refresh_token.clone();
        if let Ok(out) = serde_json::to_string_pretty(&stored) {
            let _ = std::fs::write(&path, out);
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
    /// 200. WHEN THE TOKEN DIES. GoTrue sends both of these and we
    /// ignored both; either one answers the question.
    ///
    /// BOTH ARE READ BECAUSE THE FAILURE MODE IS BAD. If only
    /// `expires_in` came back and this were the only field, every
    /// session would carry zero, `needs_refresh` would say yes to
    /// every call, and the app would make a round trip to the auth
    /// server BEFORE EVERY SINGLE DATABASE READ. Safe, and unusable.
    /// Reading the other one costs a line.
    expires_at: Option<i64>,
    expires_in: Option<i64>,
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

    // 197. ASK WHICH PERSON THIS SIGN-IN IS, ONCE, HERE. The schema's
    // ownership columns hold a profile and GoTrue only knows about
    // sign-ins, so something has to translate. Doing it at sign-in means
    // the nine places that write an owner take it off the session
    // instead of each deciding, which is the shape that let the two be
    // confused in the first place.
    //
    // A HARD ERROR RATHER THAN A FALLBACK TO `user_id`. The two are
    // equal today, so falling back would work and would keep working
    // until the first linked account, at which point it would silently
    // write rows owned by nobody. If we cannot tell who you are, we
    // must not let you write rows owned by a guess.
    let profile_id = resolve_profile(&access_token)?;

    Ok(Session {
        access_token,
        refresh_token: parsed.refresh_token.unwrap_or_default(),
        user_id: user.id,
        profile_id,
        email: user.email.unwrap_or_default(),
        // Prefer the absolute time; fall back to the relative one.
        // Zero only if neither arrived, which means "renew now".
        expires_at: parsed
            .expires_at
            .or_else(|| parsed.expires_in.map(|secs| now_secs() + secs))
            .unwrap_or(0),
    })
}

/// The person behind a sign-in, from `current_profile()` (197).
///
/// PostgREST returns a scalar-returning function's result as a bare
/// JSON value, so a uuid arrives as a string rather than a row.
fn resolve_profile(token: &str) -> Result<String, String> {
    let v = rpc(token, "current_profile", &json!({}))?;
    match v.as_str() {
        Some(id) if !id.is_empty() => Ok(id.to_string()),
        _ => Err(format!(
            "signed in, but could not resolve which profile this sign-in belongs to: {}",
            v
        )),
    }
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

/// PATCH that applies only if the row still looks the way the caller
/// last read it, and says whether it matched.
///
/// 120. FOR A COUNTER THAT IS READ BEFORE IT IS WRITTEN. Spending a
/// spell slot or a feature use is read-check-write, and writing an
/// ABSOLUTE number means two presses that read the same value both
/// write the same value: two things spent, one counted. Putting the
/// value you read into the filter - `spent=eq.3` - makes the second
/// write match no row at all.
///
/// FALSE IS NOT AN ERROR, it is the collision. The caller decides what
/// to do about it, because "no row yet" and "somebody beat me" are the
/// same empty answer and only the caller knows which is possible.
pub fn rest_update_if(
    token: &str,
    path: &str,
    filter: &[(&str, &str)],
    body: &Value,
) -> Result<bool, String> {
    let rows = rest_update(token, path, filter, body)?;
    Ok(rows.as_array().map(|a| !a.is_empty()).unwrap_or(false))
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

    /* ---------- 200. when to renew the access token ---------- */

    #[test]
    fn a_token_with_an_hour_left_is_left_alone() {
        let now = 1_000_000;
        assert!(!needs_refresh(now + 3600, now));
    }

    #[test]
    fn a_token_inside_the_margin_is_renewed_before_it_dies() {
        // THE POINT OF THE MARGIN. Still valid, and not valid for long
        // enough to survive the round trip that is about to use it.
        let now = 1_000_000;
        assert!(needs_refresh(now + REFRESH_MARGIN_SECS - 1, now));
        assert!(needs_refresh(now + 1, now), "one second left is not enough");
    }

    #[test]
    fn exactly_at_the_margin_renews() {
        // The boundary is inclusive on purpose: being wrong here should
        // cost an early refresh, never a dead token.
        let now = 1_000_000;
        assert!(needs_refresh(now + REFRESH_MARGIN_SECS, now));
    }

    #[test]
    fn inside_the_margin_a_token_wants_renewing_and_still_works() {
        // 201. BOTH TRUE AT ONCE, which is what lets a failed refresh
        // fall back instead of failing the user's read.
        let now = 1_000_000;
        let expiring = now + 60;
        assert!(needs_refresh(expiring, now));
        assert!(still_usable(expiring, now));
    }

    #[test]
    fn past_expiry_there_is_nothing_to_fall_back_to() {
        let now = 1_000_000;
        assert!(!still_usable(now - 1, now));
        assert!(!still_usable(now, now), "the exact second of expiry is gone");
    }

    #[test]
    fn an_undated_token_is_never_treated_as_usable() {
        // A zero means we were told nothing. Both questions have to
        // read it the same way or a token with no expiry would be
        // refreshed forever and fallen back on forever.
        let now = 1_000_000;
        assert!(needs_refresh(0, now));
        assert!(!still_usable(0, now));
    }

    #[test]
    fn an_expired_token_is_renewed() {
        let now = 1_000_000;
        assert!(needs_refresh(now - 1, now));
        assert!(needs_refresh(now - 86_400, now));
    }

    #[test]
    fn an_unknown_expiry_renews_rather_than_hoping() {
        // Zero is what a session from an older build carries, and what
        // an auth response without the field leaves behind. Treating
        // it as "probably fine" would rebuild the very bug this
        // closes, and the cost of being wrong is one extra refresh.
        let now = 1_000_000;
        assert!(needs_refresh(0, now));
        assert!(needs_refresh(-1, now));
    }

    #[test]
    fn either_expiry_field_answers_and_neither_means_renew_now() {
        // THE FAILURE THIS GUARDS. If only `expires_in` came back and
        // the code read only `expires_at`, every session would carry
        // zero and the app would hit the auth server before every
        // database read - safe, and unusable.
        let now = now_secs();

        let absolute: AuthResponse = serde_json::from_str(
            r#"{"access_token":"a","refresh_token":"r","expires_at":99,"user":null}"#,
        ).unwrap();
        assert_eq!(absolute.expires_at, Some(99));

        let relative: AuthResponse = serde_json::from_str(
            r#"{"access_token":"a","refresh_token":"r","expires_in":3600,"user":null}"#,
        ).unwrap();
        assert_eq!(relative.expires_at, None);
        assert_eq!(relative.expires_in, Some(3600));
        // The fallback turns it into an absolute time in the future.
        let derived = relative.expires_in.map(|s| now + s).unwrap();
        assert!(!needs_refresh(derived, now), "an hour out must not renew");

        let neither: AuthResponse = serde_json::from_str(
            r#"{"access_token":"a","refresh_token":"r","user":null}"#,
        ).unwrap();
        assert_eq!(neither.expires_at.or(neither.expires_in), None);
    }

    #[test]
    fn the_margin_is_long_enough_to_be_worth_having() {
        // A margin under a round trip is no margin. Pinned so nobody
        // tunes it to something that cannot do its job.
        // 201. A `const` BLOCK AND NOT A PLAIN ASSERT, on clippy's
        // advice and it is right: these are compile-time facts, so a
        // bad margin should fail the BUILD rather than wait for anyone
        // to run the tests.
        const { assert!(REFRESH_MARGIN_SECS >= 30) };
        // AND THE OTHER END, learned the hard way. A margin at or over
        // the hour GoTrue gives an access token means every single read
        // refreshes first, and the auth rate limiter stops the app
        // dead. The test exists because this was actually done.
        const {
            assert!(
                REFRESH_MARGIN_SECS < 3_600,
                "a margin over the token lifetime refreshes on every call"
            )
        };
    }

    #[test]
    fn a_session_without_the_field_still_deserialises() {
        // DEFENSIVE, AND SAID SO. Nothing writes a Session to disk
        // today - the fast-login file stores `pin::Stored`, not this -
        // so there is no old shape in the wild to read. The default is
        // here because `Session` is Deserialize and crosses to the
        // frontend, and because the zero it produces means "renew
        // now", which is the safe direction. The test pins that
        // direction rather than guarding a file that exists.
        let old = r#"{"access_token":"a","refresh_token":"r",
                      "user_id":"u","profile_id":"p","email":"e@x"}"#;
        let s: Session = serde_json::from_str(old).expect("old shape must still read");
        assert_eq!(s.expires_at, 0);
        assert!(needs_refresh(s.expires_at, now_secs()));
    }

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

    /// 153. AND THE NAMED FORM, which is what every embed uses now. The
    /// `!constraint` disambiguation is the fix for a pair of tables with
    /// more than one foreign key between them, so the one guard standing
    /// in front of every query has to let the `!` through untouched.
    #[test]
    fn a_named_relationship_survives_the_guard() {
        let q = [(
            "select",
            "id,character_id,characters!encounter_actors_character_id_fkey(game_id)",
        )];
        assert!(check_query(&q).is_ok());
        assert_eq!(tidy_select(&q)[0].1, q[0].1);
    }

    /// The wrapped, indented version the log really sends - the run of
    /// indentation in the middle is the typo `tidy_select` exists to
    /// repair, and it must not eat the relationship name with it.
    #[test]
    fn a_named_relationship_survives_a_wrapped_select() {
        let q = [(
            "select",
            "id,round,\
             rolls!rolls_action_id_fkey(id,role,total)",
        )];
        assert!(check_query(&q).is_ok());
        assert_eq!(
            tidy_select(&q)[0].1,
            "id,round,rolls!rolls_action_id_fkey(id,role,total)"
        );
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
