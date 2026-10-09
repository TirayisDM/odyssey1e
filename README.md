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
cargo test           # 1046 tests, about a second
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
backwards is the standard first-day confusion. There are 144 of them,
all registered in `lib.rs`.

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

#### A person is not a sign-in (197)

**Every ownership column holds a PROFILE, never an `auth.users.id`.**
Those are different things and the difference is load-bearing:

| | |
|---|---|
| a **person** | `profiles.id` — what `owner_uid`, `dm_uid` and `game_members.profile_id` hold |
| a **sign-in** | `auth.users.id` — one way of proving you are that person, and replaceable |
| the **bridge** | `identities(auth_uid → profile_id)` — the only place that knows the two are related |

Policies call **`current_profile()`**, not `auth.uid()`. `auth.uid()`
answers "which sign-in"; a policy is asking "which person". Until 197
`profiles.id` referenced `auth.users(id)`, so one person could not hold
two sign-ins and the login system could not be replaced without
rewriting ownership of every row. That FK is gone: `auth.users` now
cascades into `identities` and stops there.

**In Rust, `Session` carries both** `user_id` (the sign-in) and
`profile_id` (the person), resolved once at sign-in. **Ownership writes
take `profile_id`.** They hold equal values today, because every account
has exactly one sign-in, which is exactly why writing the wrong one is
invisible until it isn't.

Reading a *fellow member's* `display_name` is allowed by
`shares_a_game()` (198) and is symmetric — a player must be able to name
the DM, because an initiative strip that says who is up has to say it
with a name. A profile in no game of yours stays unreadable.

### Two traps, both already paid for

Full reasoning is in the migration files; the short version:

1. **A column default runs as the caller.** Revoking `EXECUTE` on a
   function used in a `DEFAULT` silently breaks every insert. Only
   trigger bodies are exempt. (003)
2. **`INSERT ... RETURNING` runs the SELECT policy too, before AFTER
   triggers fire.** Any table whose visibility depends on a row an AFTER
   trigger creates will fail in a way that blames the insert. (004)

### SECURITY DEFINER, and the hole this section used to invite

RLS is the wall. **Every `SECURITY DEFINER` function is a door through
it**, because that is what the words mean: run as the owner and do not
consult the policies. There are 34 of them; 10 are callable rather than
trigger bodies.

**This section used to say six definer warnings "must not be fixed" and
leave it there.** 193 found what that framing missed: three of those
functions — `copy_kit`, `instantiate_character`, `instantiate_npc` —
carried the *default PUBLIC grant* and checked nothing about the caller,
so **anyone holding the publishable key could create characters and NPCs
in any game whose UUID they knew, without signing in.** The definer flag
was never the problem. The missing caller check and the open grant were.

So the rule is not "definer is fine", it is:

> A definer function is safe when **either** nothing can reach it
> **or** it asks who is calling. Never neither.

Where the 10 callable ones stand:

| | |
|---|---|
| `current_profile`, `is_game_member`, `is_game_dm`, `shares_a_game`, `join_game`, `instantiate_character`, `instantiate_npc` | reachable by `authenticated`, and **each resolves the caller** rather than taking a parameter's word for it |
| `copy_kit` | **internal only** — no grant outside the owner; only `instantiate_character` calls it |
| `holder_character`, `holder_is_a_location` | reachable and deliberately **unguarded**: they are what the POLICIES call to decide who owns an entity, so a caller check inside them would consult the policies mid-flight that are calling them. They are the asking, which is why they are the one thing that cannot ask. Bounded instead by taking a bare entity UUID and returning one fact, and you only learn an entity UUID by reading `objects` through RLS |

#### Do not run this check by eye

**`select * from public.security_doors()` — an empty result is the
passing answer.** Run it after any migration that adds a function or a
table. It reports three things no constraint can: a function `PUBLIC` or
`anon` may execute, a definer function `authenticated` can reach that
never looks at the caller, and a table with RLS off or with no policy.
It is not granted to `authenticated` on purpose — the output is a map of
which functions bypass RLS, which is reconnaissance.

It has already earned this paragraph twice. It caught a false claim in
193's own header (196), and it **refused 197 outright** because the two
functions that migration creates were born `anon`-executable.

#### Functions are born closed, mechanically

`ALTER DEFAULT PRIVILEGES` **does not do this job** — verified, not
assumed. Supabase ships its own default granting EXECUTE to `anon`, and
the built-in PUBLIC grant for functions cannot be revoked that way at
all. 002 and 017 both tried; every migration since quietly undid them,
because

```sql
grant execute on function f to authenticated   -- adds a grant
                                               -- beside an open door
```

does **not** revoke the default grant to PUBLIC. The mechanism that
works is an event trigger, `functions_are_born_closed` (196), which
strips PUBLIC and `anon` from any new function in `public` as the DDL
completes. **A new definer function still needs a caller check** — the
trigger closes the grant, not the logic.

One advisor note is real and open: leaked-password protection
(HaveIBeenPwned checking on signup) is off, and would be worth turning
on before the project has users who are not you.

### The numbering, and why it has gaps

**EVERY CHANGE GETS A NUMBER, NOT EVERY CHANGE GETS A MIGRATION.** The
number is the change, and it is cited from wherever the work landed —
`-- 116.` at the top of a migration, `// 112.` in `main.js`, `/// 119.`
on a Rust method. A change that touched no schema has a number and no
file here, which is why **154 files run from 001 to 198.**

The 44 gaps are all of that kind, and the comments say so in as many
words: 070 "taught the sheet to show it" where 071 filled the column;
099 "stopped a Ny'ook writing a Strength above 13", which is a rule in
`species.rs`; 111 and 112 gave casting a target, in JavaScript and Rust;
188-192 were conditions and the scribing labels, which were all code.

```
066-070  072  079-083  090-091  099  108-112  119-120  125
133-134  138  140-141  153-154  169  173-174  176  178-182
184  188-192
```

**The chain is complete**, re-verified 2026-10-09. Every migration the
database has a record of has a file here, MD5-verified against
`supabase_migrations.schema_migrations`. The database holds 164 rows for
154 files; the difference is five migrations that were applied in parts
and committed as one file each (026-028, 031, 032, 043), not a gap.

It was not complete until 2026-10-03: seven files (101–107 — the spell
catalogue and the whole cleric list) had been applied to the live
project and never committed, the same fault 0ed7491 fixed for four
others. They were recovered from the database's own record, so they are
the files AS APPLIED rather than a reconstruction of what they probably
said.

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

About 39k lines of Rust - 26k of rules across 40 modules, 10k of
plumbing across 17 command files - with 1046 tests, 40 tables, 153
policies, and a frontend of 9.5k lines of plain JS.
`STATUS.md` is the detailed handoff; this is the shape of it.

**The app has eight top-level tabs** — Play, Run, World, Characters,
Objects, Creatures, Spells, Trade — and the character sheet has five
subtabs: Stats, Description, Equipment, Skills & Talents, Prayers.
Creatures is DM-only and is offered on `amDM()`; the access rule itself
is Postgres's.

Working, roughly in the order it was built:

* **Access** — auth, games, join-by-code, RLS verified with four
  accounts. There is no DM *account*: "dm" is a per-game role in
  `game_members.role`, so the same person is DM of one game and a player
  in another, and `is_game_dm()` backs 100 of the 153 policies
* **Creatures** — a template IS a character (`is_template`), so a
  creature is edited with the same sheet a player uses and placing one
  on the board is a copy, contents of its containers and all. `npcs` is
  the published reference you import from, stocked with 134 creatures
  across all fourteen types whose natural attacks are TECHNIQUES like
  any weapon's - a wolf reaches Snap, an owlbear reaches Crush the
  Throat, gated by level. The statblocks are SRD-derived from memory
  rather than transcribed, so spot-check one before a session leans on
  it, and the catalogue is not the whole SRD. 5e's fourteen creature
  types,
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
