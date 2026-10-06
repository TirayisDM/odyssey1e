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
cargo test           # 884 tests, about a second
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

## Working on this with someone else

**You need nothing secret.** This repo is public, and the only
credential in it is the Supabase *publishable* key, which is designed to
ship inside clients. Clone, install the prerequisites above, run it.

**Sign up in the app with your own email.** That gives you your own
account and your own campaign, not a view of somebody else's. Every
catalogue row — items, spells, species, classes, class features, skills
— is global (`game_id IS NULL`) and readable by any signed-in user, so a
brand-new account opens onto the full 5e reference data with nothing
borrowed. Create a game and you have somewhere to put characters.

You can write rules, write tests, and drive the whole app this way
without touching anybody else's data, because `games` is the tenant root
and everything under it is gated on `is_game_member`.

Two things that are **not** included and are asked for separately:

| to | you need |
|---|---|
| push to this repo | a GitHub collaborator invite, or fork and open a PR |
| apply a migration | an invite to the `Odyssey RPG` Supabase org |
| see somebody's actual campaign | their game's join code, via Join by code |

Without the Supabase invite you can still write
`supabase/migrations/NNN_name.sql` and have whoever holds the project
apply it. That is the normal arrangement and it keeps one person
accountable for the schema.

**Never put a service-role key or the database password in this repo.**
A service-role key bypasses RLS completely; `supabase.rs` says in as
many words that one appearing there is a bug.

---

## Layout

```
src/                     frontend. Vanilla JS, no framework, no build step.
  index.html             every pane, including the ones that start hidden
  main.js                ~8k lines. All of it.
  styles.css
src-tauri/src/
  *.rs                   THE RULES. Pure, tested, no Tauri, no network.
  commands/*.rs          THE PLUMBING. Tauri commands and row reading.
  supabase.rs            auth and PostgREST transport
  lib.rs                 the command registry, and the roll path
supabase/migrations/     schema, applied in order. Read the comments.
```

### The one architectural line that matters

`src/*.rs` are **rules**: pure functions over plain data, with tests, no
Tauri attributes and no network. `src/commands/*.rs` are **plumbing**:
they read rows, call a rule, and write rows back. Plumbing has no tests
because there is nothing in it to test.

The failure this prevents has happened in this codebase more than any
other: **a fact written down in more than one place.** Four places once
derived a skill modifier and three of them asked the engine; a species'
save bonus lived in the roll path and nowhere the sheet could see it.
Every time, the fix was to make one place right and have everything else
ask it. If you find yourself computing a game number in `main.js` or in
a `commands/` file, that is the smell.

A second rule with the same cause: **800 lines per file is the point to
split, not the ceiling to reach,** and a new subsystem gets a new file.

### Two rules about writing

**A button that writes goes through `guard`.** Every roll is a fresh
set of dice and a fresh row, so a second dispatch is not a harmless
duplicate - it is a second swing nobody took. `guard(el, fn)` takes an
element; `guarded(selector, fn)` is the same function with a lookup in
front. A button built per row needs the first one, which is how eight
writes went unguarded until 120.

**A counter that is read before it is written moves by compare-and-set.**
Read-check-write with an absolute number is correct exactly once:

```rust
let spent = load_slots(..)?;            // both presses read 0
may_spend_slot(level, &spent, want)?;   // both pass the check
write_slot(.., want, spent[i] + 1)?;    // both write 1
```

Two spells cast, one slot spent. `supabase::rest_update_if` puts the
value you read into the filter - `spent=eq.3` - so a write that lost
the race matches no row, and an empty result IS the collision. Spell
slots and feature uses both go through it.

### Calling Rust from JS

A `#[tauri::command]` becomes callable from the frontend. Arguments are
`snake_case` in Rust and `camelCase` in JS; Tauri converts. Getting that
backwards is the standard first-day confusion. There are about 140 of
them (136 at the time of writing), all registered in `lib.rs`.

### Testing the frontend without the app

`main.js` is loaded by a stub that fakes `window.__TAURI__`, so the real
frontend can be driven in an ordinary browser against fixtures. Copy
`src/` somewhere, inject a `stub.js` that defines
`window.__TAURI__.core.invoke`, and serve it with `python -m http.server`.
This is how the layout work gets checked at several widths without
clicking through a Tauri window.

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

Run the security advisor after every DDL change. **Six
`SECURITY DEFINER` warnings are expected and must not be "fixed":**

| | why it has to be definer |
|---|---|
| `is_game_member`, `is_game_dm` | every policy calls them; invoker rights and they cannot see the rows they are deciding about, so everything fails closed |
| `holder_character`, `holder_is_a_location` | the same, for the policies on objects |
| `join_game`, `instantiate_npc` | RPCs that write rows the caller provably cannot write yet — that is the whole job |

The first four are verifiably referenced by live policies; checking that
is one query against `pg_policies` rather than a matter of opinion.

Two other advisor notes are real and open: leaked-password protection
(HaveIBeenPwned checking on signup) is off, and would be worth turning
on before the project has users who are not you.

### The numbering, and why it has gaps

**EVERY CHANGE GETS A NUMBER, NOT EVERY CHANGE GETS A MIGRATION.** The
number is the change, and it is cited from wherever the work landed —
`-- 116.` at the top of a migration, `// 112.` in `main.js`, `/// 119.`
on a Rust method. A change that touched no schema has a number and no
file here, which is why 99 files run from 001 to 118.

The 19 gaps are all of that kind, and the comments say so in as many
words: 070 "taught the sheet to show it" where 071 filled the column;
099 "stopped a Ny'ook writing a Strength above 13", which is a rule in
`species.rs`; 111 and 112 gave casting a target, in JavaScript and Rust.

```
066-070  072  079-083  090  091  099  108-112
```

**The chain is complete.** Every migration the database has a record of
has a file here, MD5-verified against
`supabase_migrations.schema_migrations`. It was not complete until
2026-10-03: seven files (101–107 — the spell catalogue and the whole
cleric list) had been applied to the live project and never committed,
the same fault 0ed7491 fixed for four others. They were recovered from
the database's own record, so they are the files AS APPLIED rather than
a reconstruction of what they probably said.

**The habit that caused it:** applying a migration and writing the file
afterwards, which in practice means never. Write the file and apply it
in the same breath. `tools/recover_migrations.py` is there so that if it
happens again it is one command rather than an afternoon.

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

About 32k lines of Rust - 24k of rules across 37 modules, 7.5k of
plumbing across 15 command files - with 884 tests, 36 tables, and a
frontend of 8.2k lines of plain JS.
`STATUS.md` is the detailed handoff; this is the shape of it.

**The app has eight top-level tabs** — Play, Run, World, Characters,
Creatures, Objects, Cleric Prayers, Trade — and the character sheet has
five subtabs: Stats, Description, Equipment, Skills & Talents, Prayers.
Creatures is DM-only and is offered on `amDM()`; the access rule itself
is Postgres's.

Working, roughly in the order it was built:

* **Access** — auth, games, join-by-code, RLS verified with four
  accounts. There is no DM *account*: "dm" is a per-game role in
  `game_members.role`, so the same person is DM of one game and a player
  in another, and `is_game_dm()` backs 95 of the 143 policies
* **Creatures** — a template IS a character (`is_template`), so a
  creature is edited with the same sheet a player uses and placing one
  on the board is a copy, contents of its containers and all. `npcs` is
  the published reference you import from, stocked with 36 creatures
  across all fourteen types whose natural attacks are TECHNIQUES like
  any weapon's - a wolf reaches Snap, an owlbear reaches Crush the
  Throat, gated by level. 5e's fourteen creature types,
  which five seeded spells are written against. Export and import as
  a file for backup and sharing: keys travel, ids do not, and anything
  the receiving game has never heard of is skipped and listed rather
  than refusing the file
* **Dice** — the engine, with parity tests against the original
  `diceroller.js`, plus crit/fumble thresholds and formula doubling
* **Characters** — abilities, skills, saves, species (bonuses, maxima,
  unarmoured rules, traits), multiclassing, class features and the
  choices they force, Karma
* **Rolls** — a named request resolves to a formula; a roll is a record,
  snapshotted and never rewritten
* **Equipment** — a ten-slot ladder with six hip places, containers,
  weight and encumbrance, attunement, and `grants`: one vocabulary for
  everything an item does to whoever wears it
* **Combat** — attacks and techniques, initiative, action economy, death
  saves and massive damage, damage resistance/immunity/vulnerability
  from five sources - a species, a class feature, a spell, an item and a
  monster's statblock - each named on the sheet
* **Time** — one game clock in six-second ticks, short and long rests,
  hit dice, feature uses and recharges, effects that expire
* **Casting** — the cleric spell list, prepared prayers, slots by level,
  concentration, and effects that reach the dice (Bless adds its d4)
* **Trade** — merchants, pricing, haggling, coin

Not done, and the honest list is in `STATUS.md`: subclasses and their
features, spells for the other eleven classes, four species still
unseeded, and the `skill_choices` a class offers at creation are
recorded but not enforced.
