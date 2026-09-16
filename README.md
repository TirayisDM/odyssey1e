# odyssey1e

A Rust/Tauri app for a D&D 5e character sheet and roll narrator. Successor
to the Google Sheets + Apps Script + AppSheet system ("Rodnar Foundry
Sync"), which hit a ceiling that was architectural rather than fixable:
a spreadsheet acting as a message bus, and Discord posts going out over
Google's shared egress.

**Not related to `odyssey-engine`.** That one implements the HOPPER
rulebook and is a dungeon crawler. This is D&D 5e. They share patterns,
not schemas, not a repo, and not a database.

---

## Running it

```powershell
cd odyssey1e
npm install          # first time only
npm run tauri dev
```

The first `tauri dev` after a clean checkout compiles the whole Tauri
dependency tree — **ten to twenty minutes, mostly silent.** It is not
hung. Later runs take seconds.

```powershell
cd src-tauri
cargo test           # 22 dice parity tests
```

`src-tauri` is a standalone Cargo package. **Cargo commands run from
inside it**, not from the repo root.

### When it will not rebuild

```
error: failed to remove file ...\odyssey1e.exe
Access is denied. (os error 5)
```

A previous app window is still running and holding the binary. Close it,
or:

```powershell
Get-Process odyssey1e -ErrorAction SilentlyContinue | Stop-Process -Force
```

`tauri dev` hot-reloads frontend edits. A **Rust** change needs the
window closed and the command re-run.

---

## Prerequisites

* **Rust** via [rustup](https://rustup.rs) — let the installer add the
  Visual Studio C++ build tools when it offers. `link.exe not found`
  later means they are missing; it reads like a Rust problem and is not.
* **Node** (LTS). No bundler — the frontend is plain files served
  straight out of `src/`, and `withGlobalTauri` is on, so
  `window.__TAURI__.core.invoke` works with no imports.

---

## Layout

```
src/                     frontend. Vanilla JS, no framework, no build step.
                         Currently an access-test rig, not the real UI.
src-tauri/src/
  lib.rs                 Tauri commands. Thin wrappers only — no rules logic.
  dice.rs                the dice engine + its tests
  supabase.rs            auth and PostgREST transport
supabase/migrations/     schema, applied in order. Read the comments.
```

### Calling Rust from JS

A `#[tauri::command]` becomes callable from the frontend. Arguments are
`snake_case` in Rust and `camelCase` in JS; Tauri converts. Getting that
backwards is the standard first-day confusion.

---

## Supabase

Project `odyssey1e`, ref `shraejtytdxmoxmxkwxq`, us-west-1. The URL and
publishable key are constants in `src-tauri/src/supabase.rs` — that key
is *meant* to ship in clients. **The security boundary is RLS, not the
key.** A service-role key in this repo would be a bug.

### The access model

`games` is the tenant root. `game_members`, `characters` and `rolls` each
carry `game_id` *and* an owner column — which campaign a row belongs to,
and who controls it, are different questions.

Verified with four accounts: a non-member sees zero rows everywhere; a
player can read another player's character but not change it; the owner
of a roll can patch it while `pending`/`resolved` and not after.

Reference data (dice art, narrative lines, catalogues) is **not**
tenanted — global, with a nullable `game_id` for campaign-specific
overrides.

### Two traps, both already paid for

Full reasoning is in the migration files; the short version:

1. **A column default runs as the caller.** Revoking `EXECUTE` on a
   function used in a `DEFAULT` silently breaks every insert. Only
   trigger bodies are exempt. (003)
2. **`INSERT ... RETURNING` runs the SELECT policy too, before AFTER
   triggers fire.** Any table whose visibility depends on a row an AFTER
   trigger creates will fail in a way that blames the insert. (004)

Run the security advisor after every DDL change. Three
`SECURITY DEFINER` warnings are expected and must not be "fixed" —
`is_game_member`, `is_game_dm` and `join_game` need `authenticated` to
hold `EXECUTE` or every policy fails closed.

---

## Principles carried over from the old system

**A roll is a record.** `character_name`, `roller_name`, the die art and
the dice set are snapshotted onto the row at insert, never derived at
read time. The AppSheet version derived them, so every historical roll
silently changed its die art whenever the player equipped a different
set — a record that rewrote itself.

**Never claim success for a side effect that did not happen.** Status is
an exact vocabulary. Write the result first, promote it only once
delivery lands.

**Never let a slow optional enrichment gate a fast required result.**
Dice first, narrative patched in when it arrives. The owner-update policy
on `rolls` exists precisely to permit that patch.

**Resolve where the user is.** Dice are rolled in Rust, on the device,
before anything crosses the network. The insert records something that
already happened. This is the thing the old architecture structurally
could not do.

---

## State

Working: auth, games, join-by-code, characters, rolls, the dice engine
with parity tests against the original `diceroller.js`.

Next: migration 005 — the character sheet, so a named request like
`insight` resolves to a formula instead of one being typed in. The seed
data (~700 rows: 360 narrative lines, 80 dice faces, skills, spells,
techniques) is inventoried in `HANDOFF_rust_port.html`.
