# odyssey1e - session handoff

**Written 2026-09-17, updated after targets and outcomes on the laptop.**
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

Character1 also carries an inventory now - the thirteen rows off the
Inventory tab of `Application Data.xlsx`, cross-checked against the
Foundry export in `_RAW`. Two items are equipped, Scale Mail and the
Mace; the Light Hammer and the Heavy Crossbow are carried, not held; the
Ember is attuned at 6 of 7 charges. Character2 carries nothing, which
makes it the empty-inventory case for free.

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

Equipment reads as it should too, and the panel is the fastest way to
see it. Character1 carries the thirteen rows off the Inventory tab. The
Mace is proficient because the source flagged it; the Light Hammer is
proficient because it matches `sim` and shows both melee and thrown; the
Heavy Crossbow is `martialR` against a character trained only in `sim`
and comes back NOT proficient, which is the case that silently moves
to-hit. Nothing on that panel is computed in JavaScript - it shows the
engine's answer and the evidence behind it.

The dice engine now takes a crit and fumble range per roll instead of
assuming 20 and 1. Deepsong Echo crits on 18, Crystal Resonance fumbles
on 1-3, and a technique with `fumble_max` 0 cannot fumble at all - the
engine marks and judges all of those correctly, and `double_dice` turns
`1d6+2` into `2d6+2` for a crit without touching the modifier. Standard
rolls render byte-identically to before, which is pinned by a test.
Nothing calls it yet; the attack key is the caller.

And a roll can now be given a target. `insight` against DC 15, or
against AC 15 labelled Goblin 1, comes back HIT or MISS with the margin
and the reason on the card. A natural 20 against an AC reads "hit on a
natural 20" even where the total fell short, because the face decided
it; the same 20 against a DC does not, which is rules as written.
Leaving the target blank behaves exactly as before and writes no
verdict at all. Verified live on Character1.

**113 tests, zero warnings.** `cd src-tauri && cargo test`.

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
007 technique item keys - techniques stop naming their weapon in prose
008 items - the item catalogue, character_items, and the two
    proficiency arrays on characters; 13 seed rows
009 roll targets - a roll carries what it was trying to beat, and
    whether it got there

All applied. Files in `supabase/migrations/`. **Read the comments** -
each one carries why it exists, and 003 and 004 are fixes for my own
mistakes with the reasoning written out.

**007 and 008 are read by `equipment.rs` now.** 007
replaced `techniques.weapon` - free text holding a display name like
'light hammer (thrown)' - with `item_key` plus `mode`. A display name is
not an identifier, and that one string was carrying two facts: which
item, and which attack mode. The light hammer has different technique
lists for melee and thrown, 7 and 6.

008 seeds `items` from the Foundry export in the `_RAW` tab of
`Application Data.xlsx`, the same source the AppSheet Inventory and
Attacks tabs were generated from. Facts only - `1d6`, never `1d6+2`.
The old Attacks tab stored to-hit and damage with the character's
modifiers already folded in, which is exactly why its own header told
you to reseed after every level-up.

Item keys are load-bearing and unprotected: `techniques.item_key`
points at `mace_of_the_deep_song`, `light_hammer` and `heavy_crossbow`
by value, with no FK to catch a typo. `check_item_keys` asserts it, and
also the stronger pair check - that every (item_key, mode) the
techniques table uses is a mode the engine actually generates for that
weapon. A key typo points at nothing; a mode mismatch points at a REAL
weapon and is simply never offered, which is worse to find. Both pass
today: 31 rows, four pairs, the light hammer splitting 7 melee and 6
thrown. The `check keys` button on the equipment panel runs it.

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

**A partial unique index cannot back a foreign key.** The consequence
of the trap above, and the reason every cross-reference between
reference tables is by key VALUE, not by FK: `dice_faces.set_key`,
`character_skills.skill_key`, and now `techniques.item_key` and
`character_items.item_key`. An FK on the surrogate id would work
mechanically and defeat the point - it would bind the row to either the
global item or one campaign's override, when the whole purpose of the
key is that it resolves to whichever the reader is entitled to see. The
cost is that nothing catches a typo, so the integrity check has to live
in Rust. (007, 008)

**Foundry spells armor two ways, and neither the data nor the database
will tell you.** The item subtype is `light`/`medium`/`heavy`/`shield`;
the character's `armorProf` is `lgt`/`med`/`hvy`/`shl`. So
`'medium' in ['lgt','med','shl']` is false while Rodnar is in fact
proficient in medium armor - no exception, no null, just a wrong
answer. It cost a bad seed generator that quietly emitted Scale Mail as
ordinary equipment with no AC and no armor category, which is valid SQL
and passes every constraint. 008 normalizes to the prof spelling in the
seed and the check constraint admits only that spelling, so the two
vocabularies meet once and the database speaks one dialect after.

The weapon side is deliberately NOT normalized: `weapon_class` keeps
`simpleM`/`martialR` because the M/R suffix decides melee or ranged and
drives the default ability, so that match is a `sim`/`mar` PREFIX test
instead. Two vocabularies, two different reconciliations, each stated
in its column comment. This asymmetry is the most likely thing to trip
the next person. (008)

**`flex:none` does not undo `width:100%`.** `styles.css` makes every
input full width, which is right for the stacked forms and wrong for a
checkbox in a row. `flex: none` is `0 0 auto`, and an `auto` basis reads
the width - so the checkbox stayed 525px wide and crushed the item name
into five wrapped lines. `width:auto` is the fix, as `.abil` already
does it. Worth knowing because it looks like a flex bug and is not.

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

## Targets and outcomes - SETTLED AND BUILT (009, resolution.rs)

A roll used to end at a number and leave a human to decide what it
meant. It can now carry a target, and when it does the engine says
whether it was met. Verified live: a natural 20 reading `auto_hit` at
margin 11, an ordinary hit, and a miss at margin -7.

**One concept, not two.** An attack targets an AC, a check or save
targets a DC, and both are `d20 + modifier` against a number.
`target_kind` records which, because exactly one rule cares.

**Two verdicts, different axes.** The FACE verdict - crit, fumble,
normal - comes off the raw d20 against the thresholds in force, and
`dice.rs` decides it. The MARGIN verdict comes off the total against
the target. They disagree often: a natural 20 hits an AC it never
reached, a natural 1 misses one it cleared. Combining them wrongly
gives correct arithmetic and wrong rules, and it is invisible in the
output because the number looks right. `resolution.rs` is the only
place they meet.

**The auto rule is AC-only.** Rules as written: a natural 20 on an
ability check is a 20 and can still fail. `TargetKind::auto_decides` is
the single line to change if the campaign wants naturals to decide
checks too - still an open house-rule question, but the code is ready
either way and a test asserts the two kinds genuinely differ.

**The target is visible.** An ordinary column under the existing roll
policies. A visible target means the dice decided; a hidden one means
the DM decided, and this is being built so the dice decide. The UI need
not put it in lights; nothing works to hide it.

**Absent is not failure.** `success` is NULL for a roll with no target,
and every roll written before 009 reads that way. A damage roll is not
a failed anything.

**`reason` is stored, not just computed.** auto_hit / auto_miss / met /
missed, so a card can say "hit on a natural 20" rather than leaving
someone to work out how 12 beat an 18. It surfaced as a dead_code
warning - the compiler pointing out that WHY had been worked out and
thrown away. Same traceability the equipment panel gives proficiency,
and what a narrator needs handed to it rather than inferred.

**Snapshots, not links.** `target_label` holds 'Goblin 1',
`target_value` holds the number. Delete that goblin or edit its AC
mid-fight and last night's rolls must not change their minds.

---

## Combat - DECIDED, NOT BUILT

The shape agreed, so the next sessions are not re-deriving it.

**The DM supplies NPCs; an encounter lists them; a player picks a
target with a button.** Theater of the mind - no positions, no ranges,
no movement. The combatant list is just a list, which strips out most
of what makes combat systems horrible.

**Statblock catalogue plus per-encounter instances**, which is
`items` + `character_items` again: the goblin's stats authored once,
'Goblin 1', 'Goblin 2', 'Goblin 3' as three instances pointing at it.
Same catalogue-plus-junction, keyed by value. Nothing new to invent.

**A hit takes HP off automatically.** That makes this a combat tracker,
not a roll logger, and it has a consequence: one swing is a to-hit roll
plus a damage roll plus an HP change, so the ACTION has to exist as a
thing that owns its parts. Otherwise "that shouldn't have hit" has
nothing to undo. The action is also the unit a narrator wants handed to
it - those two needs turn out to be the same need.

**HP as an event log, not a mutable number.** Not `hp = hp - 7` but a
row saying Goblin 1, -7, from this damage roll. Current HP is the sum.
Undo is deleting a row. A DM correction or a heal is the same shape,
another row, not a special case. And it can always answer WHY the
goblin is at 3, which a bare number never can. Same instinct as `rolls`
itself, which is already an event log.

**Player characters have no HP in the schema at all.** `characters`
has level, name, portrait, narrative pack - nothing else. The Character
tab carries HP_Current, HP_MaxOverride and HP_Temp, none of it ported.
The moment a goblin swings back that is needed, so it belongs in the
same migration as combatant HP.

Staging, each step usable on its own: the attack key with a typed
target (playable now - 009 is in); then encounters and combatants, so
the number comes from a button instead of a keyboard; then HP events,
so the hit lands; then the action grouping and the narrator on top.

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

**The equipment chain is one step from done.** 007 and 008 put the
schema in place, `equipment.rs` reads it - proficiency, attack modes,
the one-armor rule, the key integrity check - and `dice.rs` now carries
variable crit and fumble thresholds. Only the attack itself remains,
item 1 below.

**A DECISION TAKEN, NOT YET BUILT: proficiency must be visible, not
just felt.** The Heavy Crossbow comes back at +1 where the Mace comes
back at +5, and the difference is entirely whether the character is
trained. A player who cannot see that reads it as the app being wrong.
`Resolved.modifier` already exists so a UI can explain a number instead
of printing it; the attack path should carry enough to say WHY - the
ability used, the proficiency bonus applied or withheld, and the fact
that it was withheld. Traceable is the requirement, not decorative.

**That question is now answered.** 009 and `resolution.rs` are in, and
the attack key is no longer blocked - see "Targets and outcomes" above
for what was settled, and "Combat" for the shape of what follows.

Roughly in order:

1. The `attack` key in `resolve_request`. Nothing blocks it now: the
   thresholds are in `dice.rs`, the loadout is on the sheet, and a roll
   can carry a target. It should pass one for an attack - the AC comes
   from a typed number until encounters supply it from a button. Base
   weapon attack, then a
   technique lookup that replaces the damage dice and the thresholds,
   gated by `min_level`. Snapshot the result onto the roll per the
   record principle. `skill_prompts` already has its `attack` row
   seeded. `Thresholds`, `roll_formula_as` and `double_dice` are
   waiting in `dice.rs`, annotated dead_code, for exactly this caller.
   Parity fixture: `WeaponsAttacks.js` plus `_RAW` produce the four
   known-good Attacks rows - the same trick the dice engine used
   against `diceroller.js`. One caveat, the fixture has a bug: the
   Mace's reach is 8 in `_RAW`, but the activity carries
   `range.value: '5'` with `override: False` and the old script uses it
   anyway. Display only, does not touch to-hit or damage.
2. Die art on the roll - read the equipped set from character_dice,
   look up dice_faces for the natural d20, snapshot image_url and
   set_key onto the roll at insert, per the record principle.
   `narrative.rs` is the shape to copy, down to caching it on the sheet.
3. The rules modules still unported: death saves, rests, spell slots -
   each isolated in its own Apps Script file, each wants its own Rust
   module with tests. Each also wants a `resolve_request` key, and the
   vocabulary already has room for `death` and `spell`.
4. Delivery - whatever moves a resolved roll to `delivered` and puts it
   in front of the table. This is the piece that closes the loop
   AppSheet closed, and nothing else on this list matters as much.
   **It has now been deferred twice.** Note that and decide
   deliberately rather than by drift.
5. Session persistence (currently in memory - a restart signs you out;
   fine on desktop, fatal on a phone). Wants the OS keychain, not a file.
6. A real phone-first UI. What exists is a desktop test rig.
7. Android via `npm run tauri android init`. iOS needs a Mac.

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

`WeaponsAttacks.js` is the one to read before item 3 above. It is the
whole attack derivation - ability by mode, finesse picking the better of
STR and DEX, proficiency from explicit flag then weapon class - and it
is also the cautionary tale, since everything it computes it then wrote
into a spreadsheet tab that went stale on every level-up.

**`G:\My Drive\appsheet\Application Data.xlsx` is the data behind all of
it**, 22 tabs. `_RAW` holds the Foundry character export as JSON, split
across 8 cells to fit the 50k-per-cell limit - reassemble by joining the
cells over 1000 characters. That export is the honest source: unbaked
`1d6`, `properties`, `baseItem`, `weaponProf`, `armorProf`. The
`Inventory` tab is the same 13 items flattened, and `Attacks` is the 4
derived rows. Prefer `_RAW` over either; the other two have the
character's modifiers already folded in.

The generator that produced the 008 seed from `_RAW` was a throwaway
and is gone. Regenerating it is 20 lines of Python, but remember the
armor spelling trap above or it will emit Scale Mail as equipment.
