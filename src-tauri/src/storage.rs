//! Supabase Storage - where an uploaded picture's bytes actually live.
//! 284.
//!
//! THE FIRST USE OF STORAGE IN THIS PROJECT. Everything before it was
//! rows: a character, a tile, a roll. 283 stored somebody else's
//! address for a picture, which is a row too - and carries the failure
//! the panel names, that the picture disappears when that server does.
//! Dropping a file in means holding the bytes.
//!
//! THE BUCKET IS PRIVATE, which is the decision the rest of this file
//! is shaped by. A public bucket hands back a URL that works forever
//! and costs nothing to read; it is also the one thing in this
//! application that would be readable without an account, a membership
//! or a policy. 203's line is that `amDM()` is an offer and RLS is the
//! boundary. So `storage.objects` carries the same four policies
//! `character_images` does, and the price is paid here: a signed URL is
//! minted per picture on every read.
//!
//! THE PATH IS THE POLICY. `<character_id>/<16 random bytes>.<ext>`,
//! so `storage.foldername(name)[1]` is the character id and 284's
//! policies can join to `characters` exactly as the table's do. The
//! random half is only there so two files named `portrait.png` do not
//! collide.

use serde_json::{json, Value};

use crate::supabase::{self, SUPABASE_ANON_KEY, SUPABASE_URL};

pub const BUCKET: &str = "character-images";

/// Two megabytes. A pasted screenshot off a 4K monitor is comfortably
/// more than this, which is the case the cap exists for - and the
/// panel says the number BEFORE you drop rather than after, because a
/// limit you learn by failing is a limit nobody knew about.
///
/// Said in three places on purpose: here, on the bucket (284), and in
/// the panel. The bucket's is the one that cannot be forgotten.
pub const MAX_BYTES: usize = 2 * 1024 * 1024;

/// How long a signed URL lasts. A day, because the alternative failure
/// is a window left open overnight showing broken circles in the
/// morning, and these are pictures of elves rather than bank
/// statements.
pub const SIGNED_FOR: i64 = 60 * 60 * 24;

/// WHAT A FILE IS, read from its own first bytes.
///
/// NOT FROM WHAT THE DROP CLAIMED. `File.type` is whatever the
/// operating system guessed from the extension, and the clipboard will
/// say `image/png` about anything. The four formats below are the four
/// the bucket allows; everything else is refused by name so somebody
/// dropping a PDF is told what happened.
pub fn sniff(bytes: &[u8]) -> Option<(&'static str, &'static str)> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        return Some(("image/png", "png"));
    }
    if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        return Some(("image/jpeg", "jpg"));
    }
    // RIFF....WEBP - the four bytes between are the length, which is
    // why this is not one contiguous compare.
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some(("image/webp", "webp"));
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some(("image/gif", "gif"));
    }
    None
}

/// `<character_id>/<32 hex characters>.<ext>` - see the module header
/// for why the first segment matters.
pub fn object_name(character_id: &str, ext: &str) -> String {
    use rand::Rng;
    let mut raw = [0u8; 16];
    rand::thread_rng().fill(&mut raw[..]);
    let mut hex = String::with_capacity(32);
    for b in raw {
        hex.push_str(&format!("{:02x}", b));
    }
    format!("{}/{}.{}", character_id, hex, ext)
}

fn object_url(name: &str) -> String {
    format!("{}/storage/v1/object/{}/{}", SUPABASE_URL, BUCKET, name)
}

/// Put the bytes there. Fails if something is already at that name -
/// `x-upsert: false` - because a collision would mean the random half
/// of the name repeated, and quietly overwriting somebody's portrait
/// is a worse answer than an error nobody will ever see.
pub fn upload(token: &str, name: &str, bytes: Vec<u8>, content_type: &str) -> Result<(), String> {
    let resp = supabase::http()
        .post(object_url(name))
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", content_type)
        .header("x-upsert", "false")
        .body(bytes)
        .send()
        .map_err(|e| format!("could not reach Supabase Storage: {}", e))?;

    let status = resp.status().as_u16();
    if (200..300).contains(&status) {
        return Ok(());
    }
    let text = resp.text().unwrap_or_default();
    Err(storage_message(status, &text))
}

/// A URL the webview can put in an `<img src>`, good for `SIGNED_FOR`
/// seconds.
///
/// The REST API answers with a PATH rather than a whole address -
/// `/object/sign/bucket/name?token=...` - so it is joined back onto the
/// project URL here. Both spellings of the key are accepted because the
/// JS client normalises it to `signedUrl` and the REST endpoint returns
/// `signedURL`, and being wrong about which would read as every picture
/// silently vanishing.
pub fn signed_url(token: &str, name: &str) -> Result<String, String> {
    let resp = supabase::http()
        .post(format!(
            "{}/storage/v1/object/sign/{}/{}",
            SUPABASE_URL, BUCKET, name
        ))
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({ "expiresIn": SIGNED_FOR }))
        .send()
        .map_err(|e| format!("could not reach Supabase Storage: {}", e))?;

    let status = resp.status().as_u16();
    let text = resp.text().unwrap_or_default();
    if !(200..300).contains(&status) {
        return Err(storage_message(status, &text));
    }
    let v: Value = serde_json::from_str(&text)
        .map_err(|e| format!("bad JSON from Supabase Storage: {}", e))?;
    let path = v
        .get("signedURL")
        .or_else(|| v.get("signedUrl"))
        .and_then(|x| x.as_str())
        .ok_or_else(|| "Storage signed nothing".to_string())?;
    Ok(join_signed(path))
}

/// The path back onto the project, tolerating a leading slash or not.
pub fn join_signed(path: &str) -> String {
    if path.starts_with("http://") || path.starts_with("https://") {
        return path.to_string();
    }
    let sep = if path.starts_with('/') { "" } else { "/" };
    format!("{}/storage/v1{}{}", SUPABASE_URL, sep, path)
}

/// Take the bytes away again.
///
/// A MISSING OBJECT IS NOT AN ERROR HERE. This runs when a row is
/// deleted, and a row whose file is already gone should still be
/// removable - otherwise one failed upload leaves a row nobody can get
/// rid of.
pub fn remove(token: &str, name: &str) -> Result<(), String> {
    let resp = supabase::http()
        .delete(object_url(name))
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .map_err(|e| format!("could not reach Supabase Storage: {}", e))?;

    let status = resp.status().as_u16();
    if (200..300).contains(&status) || status == 404 {
        return Ok(());
    }
    let text = resp.text().unwrap_or_default();
    Err(storage_message(status, &text))
}

/// Storage answers with `{"error":..,"message":..}` rather than
/// PostgREST's shape, so `supabase::error_message` would print the raw
/// body. A 403 here means the policy refused, which is the one a reader
/// most needs told plainly.
fn storage_message(status: u16, body: &str) -> String {
    let said = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| {
            v.get("message")
                .or_else(|| v.get("error"))
                .and_then(|m| m.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| body.chars().take(200).collect());

    match status {
        401 | 403 => format!("Storage refused that: {}", said),
        413 => "that picture is too big for the bucket".to_string(),
        _ => format!("Storage said {}: {}", status, said),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_formats_are_read_from_their_own_bytes() {
        assert_eq!(
            sniff(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0]),
            Some(("image/png", "png"))
        );
        assert_eq!(
            sniff(&[0xff, 0xd8, 0xff, 0xe0, 0, 0]),
            Some(("image/jpeg", "jpg"))
        );
        assert_eq!(sniff(b"GIF89a....."), Some(("image/gif", "gif")));
        let mut webp = b"RIFF".to_vec();
        webp.extend_from_slice(&[0, 0, 0, 0]);
        webp.extend_from_slice(b"WEBP");
        assert_eq!(sniff(&webp), Some(("image/webp", "webp")));
    }

    // A PDF named portrait.png is the case the sniff exists for.
    #[test]
    fn anything_else_is_refused() {
        assert_eq!(sniff(b"%PDF-1.7"), None);
        assert_eq!(sniff(b"<svg xmlns="), None);
        assert_eq!(sniff(b""), None);
        assert_eq!(sniff(&[0x89, b'P', b'N']), None);
        // RIFF without WEBP is a wav file.
        let mut wav = b"RIFF".to_vec();
        wav.extend_from_slice(&[0, 0, 0, 0]);
        wav.extend_from_slice(b"WAVE");
        assert_eq!(sniff(&wav), None);
    }

    #[test]
    fn the_character_id_leads_the_object_name() {
        let n = object_name("abc-123", "png");
        assert!(n.starts_with("abc-123/"));
        assert!(n.ends_with(".png"));
        // 32 hex characters between the slash and the dot.
        let middle = n.trim_start_matches("abc-123/").trim_end_matches(".png");
        assert_eq!(middle.len(), 32);
        assert!(middle.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(object_name("abc-123", "png"), n);
    }

    #[test]
    fn a_signed_path_is_joined_back_onto_the_project() {
        assert_eq!(
            join_signed("/object/sign/character-images/x.png?token=t"),
            format!(
                "{}/storage/v1/object/sign/character-images/x.png?token=t",
                SUPABASE_URL
            )
        );
        assert_eq!(
            join_signed("object/sign/x.png?token=t"),
            format!("{}/storage/v1/object/sign/x.png?token=t", SUPABASE_URL)
        );
        assert_eq!(
            join_signed("https://elsewhere.example/x.png"),
            "https://elsewhere.example/x.png"
        );
    }
}
