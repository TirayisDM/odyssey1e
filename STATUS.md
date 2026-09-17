# odyssey1e - session handoff

**Written 2026-09-17, updated after 006 on the desktop.** Read `README.md`
first for how to run it; this
file is only where things stand and what comes next.

---

## Machines

| | |
|---|---|
| Desktop (execution) | `C:\Users\tiray\Dev\odyssey1e` |
| Laptop (planning) | `C:\Users\tiray\odyssey1e` |
| Remote | `github.com/TirayisDM/odyssey1e` (private) |

Both on Rust 1.98.1. Commit on either, `git pull` on the other. Same
two-machine pattern as `odyssey-engine`.

**Not related to `odyssey-engine`** - that implements the HOPPER
rulebook. This is D&D 5e. Shared patterns, nothing else.

## Supabase

Project `odyssey1e`, ref `shraejtytdxmoxmxkwxq`, us-west-1. URL and
publishable key are constants in `src-tauri/src/supabase.rs`. A fresh
clone talks to the same database with no setup.

Test accounts, all `tirayis.dm+<x>@gmail.com` with a password you set in
the dashboard: `dm`, `p1`, `p2`, `p3`, `p4`. Display names are set.
Existing data: one game `testdeck1` (code `Z2ZYSYVQ`), one character
`Character1` owned by p1, a handful of rolls.

---

## What works, end to end

Auth, games, join-by-code, characters, rolls. The dice engine. The
character sheet resolver.

Type `insight`, pick Advantage, hit Roll: the app reads the sheet, finds
Insight keys off WIS, adds the ability modifier and the proficiency
bonus derived from level, builds `2d20kh1+6`, rolls both d20s on the
device, keeps the higher, and logs the result. Verified live.

**40 tests, zero warnings.** `cd src-tauri && cargo test`.

The access model was tested with four real accounts: a non-member sees
zero rows everywhere; a player can read another player's character but
not change it; a roll's owner can patch it while `pending`/`resolved`
and not after.

## Migrations

001 identity and access - the tenant model and every RLS policy
002 tighten function grants - lock down the SECURITY DEFINER surface
003 join code by trigger - fixes a bug 002 introduced
004 dm reads own game - fixes an INSERT...RETURNING visibility bug
005 character sheet - abilities, skill proficiency, skill catalogue
006 reference data - narrative lines, dice sets and faces, character
    dice junction, skill prompts, spells, techniques; 533 seed rows

All applied. Files in `supabase/migrations/`. **Read the comments** -
each one carries why it exists, and 003 and 004 are fixes for my own
mistakes with the reasoning written out.

---

## Traps already paid for

Do not rediscover these.

**A column default runs as the caller.** Revoking EXECUTE on a function
used in a DEFAULT silently breaks every insert. Only trigger bodies are
exempt from the EXECUTE check. (003)

**`INSERT ... RETURNING` runs the SELECT policy too**, before AFTER
triggers fire. PostgREST always sends `Prefer: return=representation`,
so every insert is a RETURNING. Any table whose visibility depends on a
row an AFTER trigger creates will fail in a way that blames the insert.
(004)

**A primary key cannot contain a nullable column.** The nullable-tenancy
pattern (global rows with `game_id IS NULL`, overrides with it set)
needs a surrogate id plus two *partial* unique indexes - `NULL != NULL`
in a unique constraint, so without the partial index two global rows
both pass. Every reference table from 006 on hits this. (005)

**A `.ps1` must be pure ASCII** or saved with a BOM. PowerShell 5.1
decodes a BOM-less file as Windows-1252, and an em dash becomes a smart
quote, which PowerShell honours as a string delimiter. The parse error
then points well past the actual damage.

**Text ordering differs between Postgres and everything else.** The
default collation sorts 'base' before 'Rodnar'; Python and C sort the
other way. A checksum over ORDER BY text will not match a checksum
computed elsewhere unless you `collate "C"`. Cost twenty minutes in 006.

**Three SECURITY DEFINER advisor warnings are expected.**
`is_game_member`, `is_game_dm`, `join_game` need `authenticated` to hold
EXECUTE or every policy fails closed. Do not "fix" them. Run the
advisor after every DDL change anyway.

---

## Architecture decisions that should hold

**A roll is a record.** `character_name`, `roller_name`, die art and
dice set are snapshotted onto the row at insert, never derived at read
time. The AppSheet version derived them, so historical rolls silently
changed their die art whenever the player equipped a different set.

**Rules live in Rust, facts live in Postgres.** Proficiency bonus and
ability modifiers are computed in the engine, not stored or generated -
one rule in two places drifts.

**Resolve where the user is.** Dice roll on the device before anything
crosses the network. The insert records something that already happened.

**Never let a slow optional enrichment gate a fast required result.**
Dice first, narrative patched in later. The owner-update policy on
`rolls` exists to permit exactly that patch.

**Reference data is not tenanted** - global, with a nullable `game_id`
for campaign-specific overrides.

---

## Pick up here

006 is applied and every seeded table was checksum-verified against the
spreadsheet. Read the 006 header: spells and techniques were ported
AS-IS with one character's numbers baked in (spell_atk +7, DC 15,
2d8+4), flagged BAKED in their column comments, and the Spellbook
`Prepared` column was deliberately not ported. `characters.narrative_pack`
(default 'base') picks a narrative pack. `character_dice` is the
ActorDice junction, with a partial unique index enforcing one equipped
set per character.

Nothing in Rust knows about the new tables yet. Roughly in order:

1. Narrative lines wired in, so a roll card carries prose again - read
   narrative_lines for (character.narrative_pack, request key), fall
   back to 'base', pick one at random on the device, patch it onto
   rolls.narrative after the insert (owner-update policy permits this)
2. Die art on the roll - read the equipped set from character_dice,
   look up dice_faces for the natural d20, snapshot image_url and
   set_key onto the roll at insert, per the record principle
3. The rules modules still unported: death saves, rests, spell slots,
   techniques with custom crit ranges - each isolated in its own Apps
   Script file, each wants its own Rust module with tests
4. Session persistence (currently in memory - a restart signs you out;
   fine on desktop, fatal on a phone)
5. A real phone-first UI. What exists is a desktop test rig.
6. Android via `npm run tauri android init`. iOS needs a Mac.

## Loose ends

- Leaked password protection is off in Supabase auth. Turn it on before
  real players have passwords.
- The old AppSheet system is still live and still has the outstanding
  items in `ISSUES_appsheet_audit.html`. Decide whether it is being
  maintained or retired.
- Old Supabase projects `osddb`, `OSDUNGEON`, `ActiveCore_Components`
  are all paused. Free tier allows two active at a time; restoring one
  may force a choice.

## Source material

The Apps Script system it came from is in
`G:\My Drive\appsheet\raw backup` - 17 `.js` files. `diceroller.js` and
`characternarrative.js` are the ones still being ported from.
`HANDOFF_rust_port.html` and `ISSUES_appsheet_audit.html` live there and
in the claude.ai project.
