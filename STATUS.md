# odyssey1e - session handoff

**Written 2026-09-17, updated after the narrative wiring on the desktop.**
Read `README.md` first for how to run it; this
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
Existing data: one game `testdeck1` (code `Z2ZYSYVQ`), two characters
owned by p1, and a pile of rolls.

`Character1` and `Character2` are deliberate twins - same level, same
ability scores, same Insight proficiency. The only difference is
`narrative_pack`: Character1 is on `Rodnar Shieldcrest`, Character2 on
`base`. Roll the same Insight on each and the dice math is identical
while the prose is not, which is the fastest way to see the pack
precedence working. Delete Character2 if it is in the way; the cascade
takes its abilities and skills with it.

---

## What works, end to end

Auth, games, join-by-code, characters, rolls. The dice engine. The
character sheet resolver. The narrative packs.

Type `insight`, pick Advantage, hit Roll: the app reads the sheet, finds
Insight keys off WIS, adds the ability modifier and the proficiency
bonus derived from level, builds `2d20kh1+6`, rolls both d20s on the
device, keeps the higher, picks a line of prose out of the character's
pack, and logs all of it as one row. Verified live.

The prose is the part the port was for. `Religion` typed in full,
capitalised, resolves to key `rel`, finds the pack's Religion lines, and
comes back with a line that names the character's god - while the same
request on the base pack comes back in the neutral they/their voice.
Same dice, same modifier, two narrators. Verified live on both packs.

**53 tests, zero warnings.** `cd src-tauri && cargo test`.

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

**A PostgREST `in.(...)` value is not safe to interpolate.** A pack is
named `Rodnar Shieldcrest`, with a space in it, and pack names are free
text a DM can set. A value with a space, comma, parenthesis or quote in
it has to be double-quoted in the list, with a backslash before any
literal quote or backslash. Unquoted, the filter either matches nothing
or silently means something else - and either way it looks like missing
seed data rather than a bad query. `narrative::quoted()` does this; use
it for every reference table that gets filtered by a name.

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
The rule is about *slow*, and a canned line is not slow. The pack comes
down with the sheet, so choosing a line is a local die roll and it goes
into the insert body with the dice - the row is complete the first time
anyone reads it. The owner-update policy on `rolls` still exists, and
`patch_roll_narrative` is still there, for the AI-written narrative,
which is the enrichment the rule was always about.

**A pack wins whole, an override wins by line.** Two precedences, and
confusing them is the bug. Inside a (pack, key), a game-scoped row
shadows the global row with the *same seq* - a DM rewrites line 4 and
keeps the other nine. Across packs, if the character's pack has any line
for a key then `base` is not consulted for that key at all. Splicing a
voiced line in beside a neutral one inside one key reads as two
narrators, which is worse than having fewer lines.

**Roll keys are engine vocabulary.** `rolls.request` is what the player
typed; the key is what the engine resolved it to. `Insight`, `insight`
and `ins` all land on `ins`. That vocabulary - skill keys, `wis_save`,
`wis_check`, `custom`, and `attack`, `spell`, `death` when they arrive -
is what `narrative_lines.key` and `skill_prompts.key` are written in, so
it is the join between a request and both its prose and its future AI
prompt. `resolve_request` emits the full vocabulary even where nothing
is seeded for it, so a pack can grow a key without a Rust change.

**Reference data is not tenanted** - global, with a nullable `game_id`
for campaign-specific overrides.

---

## Pick up here

**Be clear about what is and is not done.** The foundation is square:
the access model, the dice, the sheet resolver, the prose. What AppSheet
did that this does not do yet is *deliver*. `rolls.status` goes
`pending -> resolved -> delivered` and nothing in this codebase moves a
row to `delivered`. There is no die art, no death saves, no rests, no
spell slots, no techniques, no session that survives a restart, and the
UI is a test rig. The port is not nearly finished; the part that had to
be right first is.

006 is applied and every seeded table was checksum-verified against the
spreadsheet. Read the 006 header: spells and techniques were ported
AS-IS with one character's numbers baked in (spell_atk +7, DC 15,
2d8+4), flagged BAKED in their column comments, and the Spellbook
`Prepared` column was deliberately not ported. `character_dice` is the
ActorDice junction, with a partial unique index enforcing one equipped
set per character. `narrative_lines` is wired in; the rest is not.

Roughly in order:

1. Die art on the roll - read the equipped set from character_dice,
   look up dice_faces for the natural d20, snapshot image_url and
   set_key onto the roll at insert, per the record principle. This is
   the closest sibling to what just landed; `narrative.rs` is the shape
   to copy, down to caching it on the sheet.
2. The rules modules still unported: death saves, rests, spell slots,
   techniques with custom crit ranges - each isolated in its own Apps
   Script file, each wants its own Rust module with tests. Each also
   wants a `resolve_request` key, and the vocabulary already has room
   for `death`, `spell` and `attack`.
3. Delivery - whatever moves a resolved roll to `delivered` and puts it
   in front of the table. This is the piece that closes the loop
   AppSheet closed, and nothing else on this list matters as much.
4. Session persistence (currently in memory - a restart signs you out;
   fine on desktop, fatal on a phone). Wants the OS keychain, not a file.
5. A real phone-first UI. What exists is a desktop test rig.
6. Android via `npm run tauri android init`. iOS needs a Mac.

Two small things worth doing while they are cheap: `preview_request`
calls `load_sheet`, so hovering a button reads the whole pack it has no
use for - a `load_sheet_lite` would fix it, at the cost of two loaders
that can drift. And `load_sheet` runs on every roll, so the pack read
sits ahead of the dice; it is one small select and it has not been worth
fixing yet, but that is where the latency is if it ever matters.

## Loose ends

- **Rotate `ROLLLOG_WEBHOOK_SECRET`** in the old Apps Script project's
  Script Properties and in the AppSheet webhook body. The value was
  visible in screenshots during a chat session, so treat it as public
  until it has been changed in both places.
- **Delete the throwaway Discord webhook** that was pasted into that
  same session. It was never used, but the URL is in a transcript and a
  Discord webhook URL is the whole credential.
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
`characternarrative.js` are the ones still being ported from. The
NarrativePacks half of `characternarrative.js` is done and lives in
`narrative.rs`; the AI-generated half is not, and the `skill_prompts`
table seeded in 006 is what it will read.
`HANDOFF_rust_port.html` and `ISSUES_appsheet_audit.html` live there and
in the claude.ai project.
