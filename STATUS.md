# odyssey1e - session handoff

**Written 2026-09-17, updated after the NPC view.**
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

Two encounters, and they are a useful pair rather than clutter. One is
`active` and one is not, which is the state that catches anything
confusing "the encounter the DM is editing" with "the encounter the
players can see". The roster holds hand-named actors from before 018
(`Goblin 1`, `Goblin 2`) beside auto-named ones from after it
(`Goblin 0001`, `Goblin Fighter 0001`), so both naming eras are on
screen at once - the ordinal does NOT continue from the hand-named ones,
because those carry a null `name_base` and the counter cannot see them.
That is 018 behaving as written, not a bug, and a campaign started after
it never sees the mixture.

The global `goblin` statblock carries a handaxe AND a scimitar, on
purpose. Same creature, one set of scores, and the only difference
between the two weapons is three letters - `fin` on the scimitar, so it
swings on DEX 14 for +4 where the axe swings on STR 8 for +1. The
Monster Manual's number falls out of a property rather than being
copied in. See the convergence note under architecture decisions.

SINCE 022 EVERY ACTOR IS ALSO A CHARACTER ROW, `is_npc` true - five of
them at the time of writing. They do not appear in a player's character
list, which is the only thing that flag does. `Unnamed` and `Snot` are
player characters created by hand during testing and are junk; delete
them when they get in the way.

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

And a character can now be hit. The sheet header reads `AC 16 · HP 74`
for Character1 - computed from Scale Mail's 14 plus DEX +2 under the
armour's cap of 2, never from the export's flat field. Character2 wears
nothing and reads AC 12.

Encounters supply the target instead of a keyboard. One dropdown lists
the active encounter's actors and challenges with their figures -
`Goblin 1 · AC 15`, `Rodnar · AC 16`, `The iron lock · DC 15` - and
picking one supplies value, kind and label together, so a half-filled
target is impossible. Hand-typing survives for when there is no
encounter.

And a weapon is a roll. `mace of the deep song` resolves to `1d20+4`
with damage `1d6+1`, and the preview says WHY: `STR +1, prof +3`, or
`NOT proficient` on the Heavy Crossbow, whose +1 would otherwise read
as a bug. `heavy smash` swaps the damage dice and the thresholds and
leaves to-hit alone; `crystal resonance` at level 5 declines outright
rather than quietly rolling the plain weapon. Verified live against
Goblin 1.

And a swing is one thing. `heavy smash` against Goblin 1 writes an
action owning two rolls - `1d20+4` and `dmg 1d8+1` - on one card, in a
single call that lands both or neither. A miss rolls no damage at all;
an attack with no target does, because with no AC to check nobody can
say it failed to connect. A natural 1 reads "missed on a natural 1",
and a total of exactly 15 against AC 15 reads "exactly". Every roll
belongs to an action now, including a miss: it happened, and it spends
an initiative slot the same as a hit.

And the hit lands. Three Heavy Smash hits on Goblin 1 for 9, 3 and 2
took it from 7 hit points to down, each event traceable to the dice that
produced it, while Goblin 2 sat untouched at 7/7 - two instances off one
statblock bleed separately. A miss writes no event at all.

Dropping to zero does not kill. It makes a creature unconscious and
dying at AC 0, still targetable, and three death saves settle it either
way. The target list reads `Goblin 1 - down` with the tally beside it,
and a button rolls the save until initiative exists to fire it
automatically.

And the DM can build the fight from the app. Encounters, enrolment,
challenges and statblocks were authored by hand in SQL until 28d5c27;
every policy had been in place since 011 and what was missing was a way
to reach them. Twelve commands in `commands/dm.rs`, none of them in
lib.rs. Enrolling is three clicks and no typing - leave the name blank
and 018 names it `Goblin 0003`, numbered per game.

THE PANEL ONLY APPEARS FOR THE DM OF THE SELECTED GAME, which is a
courtesy and not a guard: `is_game_dm` is in 011's policy on every one
of those tables, so a player calling them is refused by Postgres.
testdeck1 belongs to the `dm` account, so the panel is absent for p1 -
correct, and worth knowing before wondering where it went.

And a monster acts. Each NPC in the roster carries a target picker and
one button per attack, DERIVED from what its statblock holds: the
handaxe offers both `handaxe` and `handaxe (thrown)` because the item
carries `thr`, and nothing in the frontend knows what a handaxe is.
Characters get no buttons - their player rolls from the sheet, which is
what having one is for.

And a monster is an individual. Enrolling a goblin makes a CHARACTER
from the statblock - its own scores, its own kit, its own hit points -
so editing the type afterwards never reaches anything already in play.
It used to: hp_current is hp_max plus the sum of damage, so raising the
goblin type from 7 to 9 healed every wounded goblin on the table,
unconscious ones included, with no warning. See 022 and the convergence
note under architecture decisions.

And a PIN unlocks the app. Four digits instead of an email and a
password, which matters because the session lives in memory and every
restart signs you out. The unlock screen names the account it is about
to open, "use password" is always available, and a wrong PIN never
reaches the network. IT IS CONVENIENCE, NOT SECURITY - pin.rs says so
at the top and the reasons are worth reading before trusting it.

And a creature can be looked at, and named. `view` on a roster row
opens THE SAME SHEET A PLAYER GETS - scores with their modifiers, hit
points, armour class, the kit with its modes and proficiency - because
022 made a monster a character and there is no second view to keep in
step. `Goblin 0003` becomes `Snaggletooth` in one call that moves the
actor's label and the character's name together; older rolls keep the
name they snapshotted, and the spent ordinal is not released, so the
next unnamed goblin is still 0004.

**177 tests, zero warnings.** `cd src-tauri && cargo test`.

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
010 character vitals - HP, death saves, exhaustion, size, and the two
    columns AC is computed FROM
011 encounters - encounters, the npc statblock catalogue, actors and
    challenges; the things a roll can be aimed at
012 actions - one swing is one thing: an action owns its rolls, and
    write_action lands all of them or none
013 hp events - the links from an action to what it aimed at and who
    swung, and hit points as a log rather than a number
014 write_action hp - the writer attaches the damage event, because the
    damage roll's id cannot be known by the caller
015 dying - zero is unconscious, not dead; death saves for NPCs too
016 actor death saves - a members policy plus a trigger, after an
    RLS-blocked UPDATE turned out to fail silently
017 tighten new function grants - the two functions 009-016 left
    outside 002's pattern; consistency, not a breach
018 npc naming - an unnamed NPC names itself <Species> <Class> 0001
019 npc gear - a statblock gets ability scores and carries real items,
    so a monster attacks through the character path
020 scimitar - the weapon that makes the goblin match its own statblock
021 write_action character_name - the name was being dropped in transit
022 instantiate on enrol - a goblin becomes an INDIVIDUAL when enrolled;
    the convergence migration
023 hp_events follow the character - one subject, now there always is one
024 name_actor after convergence - 022 made 018's naming unreachable
025 rename actor - a label and a character's name move together

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

**AN RLS-BLOCKED `UPDATE` SUCCEEDS AND DOES NOTHING.** This is the
worst one on the list, because it is silent. A row a policy hides is a
row that IS NOT THERE for an UPDATE: the statement matches zero rows,
affects zero rows, and returns without error. A blocked INSERT raises
and rolls the transaction back; a blocked UPDATE just shrugs.

`write_action` returned an action id having quietly dropped half of what
it was asked to record - two death saves were rolled on Goblin 1, a 10
and a natural 1, and its counters stayed on zero. Nothing anywhere said
so. The cause was `encounter_actors` having only a dm-updates policy
while the roll came from a player.

Any write that MUST land needs either a policy that admits the caller or
a check that it actually happened. Do not assume a successful statement
changed anything. (016)

**A write button that can fire twice will fire twice.** Every click was
writing two actions with two sets of dice - two swings nobody took, and
on a death save, two saves from one press with a natural 1 among them
worth two failures toward dead. A roll is not idempotent: a duplicate is
not a harmless repeat, it is a second event. Write buttons are guarded
now, disabled for the duration with a second dispatch dropped rather
than queued. The source of the second dispatch was never proven, only
made impossible from the UI side - if it reappears, it is deeper.

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

**The export's AC is not the character's AC.** `attributes.ac` reads
`{"calc": "default", "flat": 14}`, and the spreadsheet has an AC_Flat
column dutifully saying 14. Fourteen is not Rodnar's armour class.
`calc: "default"` means Foundry COMPUTES from equipped armour; `flat`
is consulted only when calc is `"flat"`, so 14 is a leftover in a field
nobody reads. The real answer is `base_ac + min(DEX, dex_cap)` - 15 on
the export's DEX of 13, 16 for Character1 whose DEX is 14. Copying
AC_Flat in would have had every attack on him resolve one short,
forever, with nothing anywhere to suggest it.
`rodnar_is_fifteen_not_the_fourteen_in_the_export` fails loudly if
anyone wires that field up. Same species as the baked Attacks tab: a
number that looks authoritative and is a leftover. (010)

**A shield is not a second suit of armour**, and `kind = 'armor'` alone
cannot tell them apart. `check_one_armor` counted both, so mail plus a
shield - the most ordinary loadout in the game - came back refused as
"two armors". Two separate limits now, because body armour SETS the AC
while a shield ADDS to it, and `armor_category = 'shl'` is the only
thing distinguishing them. Found by writing the AC rule, not by anyone
hitting it. (010)

**A `dex_cap` of NULL is not a cap of zero.** Light armour has no cap
and passes the whole modifier; heavy armour is `Some(0)` and passes
none. Reading NULL as zero costs a rogue their entire DEX bonus and
looks like nothing. (010)

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

**AN EXPLICIT COLUMN LIST INSIDE A WRITER FUNCTION IS A SECOND SCHEMA.**
write_action names the columns it inserts, and `character_name` was not
among them - so a name set in Rust was parsed off the payload, ignored
by the select, and replaced by 'Someone' with no error anywhere. TWO
fixes were written and committed against that defect before anyone
noticed neither worked, because both were verified by COMPILING rather
than by reading a row back. Adding a column to a table and setting it
from Rust is not enough if the value travels through a writer. (021)

**A CONSTRAINT WILL REFUSE THE MIGRATION THAT EXISTS TO SATISFY IT.**
Three migrations in a row were rejected on first attempt: two by a check
constraint forbidding exactly the row the migration was creating, one by
freeze_roll_identity. Drop before backfilling, not after. The third
refusal was correct and the backfill was wrong - a snapshot that can be
corrected later is not a snapshot. (021, 022, 023)

**A GUARD THAT LISTS THE CASES IT REFUSES WILL MISS ONE.** death_save
refused Conscious and Dead; Stable is neither, so a creature that had
finished dying could roll again and start over. And 018's naming asked
"does this actor have a character", which 022 made always true, so the
ordinal branch went unreachable and every goblin was called "Goblin".
Both were found by exercising the thing, not by reading it. (024)

**A CORRECT REFUSAL LOOKS EXACTLY LIKE A BROKEN FEATURE.** The `active`
button "did not work": the database was refusing it, correctly, because
only one encounter may be active per game. The explanation existed, was
accurate, and went to the log pane on the far side of the screen. That
pane was built to make RLS denials loud FOR SOMEONE READING THE LOG,
which is a different job from telling a DM why a click did not take.
Anything that can be refused now says so next to the button that earned
it - `dmSay`, and `tryCall` which is `call` that hands back why. This
happened twice before it was fixed; if the panel grows, it will happen
again.

**TWO IDS THAT MEAN NEARLY THE SAME THING WILL BE CONFUSED.**
`state.encounterId` is the ACTIVE encounter, owned by loadTargets - what
players aim at and what a roll is attributed to. `state.dmEncounterId`
is what the DM has open, which is usually a draft. They were one
variable for ten minutes: selecting a draft, enrolling, then refreshing
the target list snapped the selection back, and a roll taken in between
would have been filed against the wrong encounter. `dmTargets` is a
third list for the same reason. The DM panel now states which encounter
it is editing and whether anyone can see it, because an outline on a row
and a word in grey was not enough - a goblin and two challenges got
built into an ended encounter before it said so.

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

## House rules, marked so nobody takes them for 5e

Two live in `death.rs`, both deliberate:

**NPCs roll death saves.** By the book a monster simply dies at zero. A
goblin here bleeds out like anyone else, which is why
`encounter_actors` carries the same two counters `characters` has had
since 010.

**An unconscious creature has AC 0.** The book keeps its armour class
and grants attackers advantage, with melee hits inside five feet landing
as crits. Flat zero is simpler and reads plainly at the table.

Everything else in there is 5e and was deliberately not invented: ten or
better succeeds, a natural 1 is two failures, a natural 20 restores one
hit point and clears the tally, three either way settles it, being hit
while down costs a failure and two on a crit, and overflow damage
meeting the hit point maximum kills outright.

One more, in `resolution.rs`: a natural 20 does NOT carry an ability
check, only an attack. That is rules as written, and
`TargetKind::auto_decides` is the single line to change if the campaign
ever disagrees.

---

## Architecture decisions that should hold

**ONE STRUCTURE FOR CHARACTERS AND NPCS. LARGELY BUILT, 022-024.**
Separate, lighter rules for monsters were fine for basic D&D. Giving an
NPC real depth is easier if it simply follows the structure a character
already has, and the long-run simplification is the point: one set of
mechanisms to build, fix and reason about instead of two that must agree.

ENROLLING AN NPC CREATES A CHARACTER. `npcs` is purely a template now -
a pattern you make individuals from. The individual owns its scores, its
kit, its hit points and its death saves, so editing the type never
reaches anything already in play. Editing a goblin is an edit of THAT
goblin. A Goblin Elite Assassin may be a recurring type; each one
generated from it is its own creature.

    npcs              the type. Scores, kit, AC, HP - a pattern.
    characters        the individual, is_npc true. Owns everything.
    encounter_actors  who is in this fight, pointing at a character and
                      remembering which type it came from.

`npc_key` survives on the actor as PROVENANCE. Nothing is read through
it. It is the door left open for species rules to be inherited one day,
which is the one thing that might reasonably flow from a type to an
individual after the fact.

WHAT THIS ALREADY COLLAPSED, all of it debt this section used to list:

    swing()          one function for a goblin and for Rodnar
    hp_events        one subject, because there always is one now (023)
    vitals_payload   one place a death-save tally lives
    resolve_ac       three sources became two - a monster's AC arrives
                     as ac_mode 'flat' and resolves through the same
                     path a player's does. A setting, not a branch.
    load_npc_sheet   ninety lines assembling a sheet that looked like a
                     character's without being one, now a lookup
    load_targets     stopped reading `npcs` at all
    proficiency      stopped being assumed-for-monsters; the statblock's
                     assumption is written ONCE at instantiation
    characters.prof_bonus  the nullable override Sheet.prof_bonus had
                     already assumed - NULL derives from level

WHAT IS LEFT, and it is small. `npc_items` and the ability columns on
`npcs` still exist, because a TEMPLATE needs somewhere to keep its
pattern - that is no longer a parallel structure, it is a different
thing. `npcs.intl` is still ugly, and is ugly only because `int` is
reserved in SQL.

THE RULE FOR NEW WORK IS UNCHANGED: when an NPC needs something, reuse
the character mechanism. If a parallel is genuinely unavoidable, say so
in the migration rather than quietly widening the gap.

**A DECISION TAKEN, NOT YET BUILT: WHO MAY BE TARGETED IS A QUESTION
ABOUT THE ACTION, NOT ABOUT THE TARGET.** Self-targeting is allowed
everywhere and filtered nowhere. That is deliberate and it is not the
final answer.

The obvious rule - you cannot aim at yourself - is wrong the moment
healing exists, and healing is the ordinary case rather than the exotic
one. A buff, a defensive stance, a second wind and most of what a cleric
does all name the actor taking the action. Even the attack case is not
clean: hitting yourself with the pommel of your own sword is a thing a
DM might allow, and the engine has no business refusing it.

So the discriminator is the KIND of action. `Resolved.key` already
carries that vocabulary - skill keys, `wis_save`, `wis_check`, `custom`,
with room for `attack`, `spell` and `death` - and a targeting rule
belongs against it, once there is a healing or buff path for the rule to
be about. Filtering by identity in the meantime would encode the wrong
rule in the easiest place to forget it.

The NPC attack row DID filter the attacker out of its own target list
for one commit. That was a game rule decided in JavaScript while a
player's roll box, three hundred lines away, allowed the same thing -
and the disagreement is how it was noticed.

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

## Combat - BUILT, except the turn

The loop runs end to end. A DM enrols a goblin, a player picks it from
a list, names a weapon or technique, and the app rolls to-hit, decides
hit or miss against the real AC, rolls damage on a hit, doubles the
dice on a crit, takes the hit points off, and writes all of it as one
thing that can be undone in one stroke.

What was decided and held up:

**Theater of the mind.** No positions, no ranges, no movement. The
combatant list is just a list, which strips out most of what makes
combat systems horrible.

**Statblock catalogue plus per-encounter instances**, which is `items`
plus `character_items` again. One goblin row, three instances, each
bleeding separately.

**A hit takes HP off automatically**, which is what forced the action
into existence: one swing is a to-hit plus a damage roll plus an HP
change, and without a parent, "that should not have hit" has nothing to
undo. The action turned out to be the unit a narrator wants handed to
it as well - the same need twice.

**HP as an event log.** Undo is deleting a row, a heal is damage with
the sign flipped, a DM correction is another row with a note, and the
goblin at 3 can say why.

**Dropping to zero is unconscious, not dead.** Three death saves settle
it. AC 0 while down, and hitting something already down costs it a
failure rather than hit points.

WHAT IS STILL MISSING, and it is two things rather than a list:

**The turn.** `encounter_actors.initiative` is a column nothing reads. Death saves
happen on a button because there is no turn for them to happen on, and
that is the only place the current build departs from the rule as
written.

**The DM side.** Encounters, actors, challenges and statblocks are all
authored by hand in SQL. Every policy for doing it from the app is
already in place; what is missing is the screen.

---

## Pick up here

**Be clear about what is and is not done.** The foundation is square
and the combat loop runs: the access model, the dice, the sheet
resolver, the prose, equipment, attacks, hit points, dying, the DM's
screen, and monsters that are individuals rather than views of a type.

What AppSheet did that this still does not is *deliver*. `rolls.status`
goes `pending -> resolved -> delivered` and nothing in this codebase
moves a row to `delivered`. There is also no initiative, no die art, no
rests, no spell slots, and the UI is a test rig. A PIN now saves you
retyping a password, but the session still lives in memory.

006 is applied and every seeded table was checksum-verified against the
spreadsheet. Read the 006 header: spells and techniques were ported
AS-IS with one character's numbers baked in (spell_atk +7, DC 15,
2d8+4), flagged BAKED in their column comments, and the Spellbook
`Prepared` column was deliberately not ported. `character_dice` is the
ActorDice junction, with a partial unique index enforcing one equipped
set per character. `narrative_lines` is wired in; the rest is not.

**The equipment chain is done, and the DM side with it.** The attack
key landed, `equipment.rs` reads the schema, `dice.rs` carries variable
crit and fumble thresholds, and 28d5c27 gave the DM a screen for
encounters, enrolment, challenges and statblocks. A monster attacks
through the same `swing` a character does.

**A DECISION TAKEN, AND BUILT: proficiency must be visible, not
just felt.** The Heavy Crossbow comes back at +1 where the Mace comes
back at +5, and the difference is entirely whether the character is
trained. A player who cannot see that reads it as the app being wrong.
`Resolved.modifier` already exists so a UI can explain a number instead
of printing it; the attack path should carry enough to say WHY - the
ability used, the proficiency bonus applied or withheld, and the fact
that it was withheld. Traceable is the requirement, not decorative.

Built, and in three places now. The attack preview says `STR +1,
prof +3` or `NOT proficient`. The equipment panel says `proficient ·
derived` against `proficient · flagged`, so an explicit answer from the
source never looks like a computed one. And an NPC's attack buttons
carry the same, which is how the goblin's +1 handaxe beside its +4
scimitar reads as the finesse rule rather than as a mistake.

**That question is now answered.** 009 and `resolution.rs` are in, and
the attack key is no longer blocked - see "Targets and outcomes" above
for what was settled, and "Combat" for the shape of what follows.

Roughly in order:

1. INITIATIVE, and with it the turn. `encounter_actors.initiative`
   has been a column since 011 and nothing has read it yet. It is what
   turns "roll a death save" from a button into something that happens
   on a creature's own turn, which is how the rule is actually written.
   Enrolment should prompt the players to roll for it. A miss spends a
   slot exactly as a hit does, which is why every roll became an action.
2. EDITING a creature, now that viewing one works. Renaming is done;
   the rest is scores, hit points, AC and kit, all of which are plain
   columns on a character the DM already owns. Then "save as template",
   which is the same copy `instantiate_npc` does, pointed the other way
   - and it is worth doing, because building an interesting goblin in
   play and keeping it is how a DM actually works.
3. The roll-to-challenge payoff. `actions.target_challenge_id` is
   written and nothing reads it, so the iron lock still cannot say
   whether it has been picked. The derivation is a query away: a
   successful action pointing at the challenge, later than its
   `reset_at`. See 011's comment on that column.
4. Die art on the roll - read the equipped set from character_dice,
   look up dice_faces for the natural d20, snapshot image_url and
   set_key onto the roll at insert, per the record principle.
   `narrative.rs` is the shape to copy, down to caching it on the sheet.
5. The rules modules still unported: rests, spell slots - death saves
   landed with 015 -
   each isolated in its own Apps Script file, each wants its own Rust
   module with tests. Each also wants a `resolve_request` key, and the
   vocabulary already has room for `death` and `spell`.
6. Delivery - whatever moves a resolved roll to `delivered` and puts it
   in front of the table. This is the piece that closes the loop
   AppSheet closed, and nothing else on this list matters as much.
   **It has now been deferred twice.** Note that and decide
   deliberately rather than by drift.
7. Session persistence, PROPERLY. A PIN now stands in front of it, so
   a restart costs four digits instead of a password - but the session
   still lives in memory and the refresh token sits in a plain file in
   the app data directory. The real answer is unchanged and is the OS
   keychain: Credential Manager, Keychain, the Android Keystore. Then
   the token is held by the operating system, the PIN becomes a second
   factor rather than the only one, and pin.rs changes from "where the
   token lives" to "what unlocks it". Read pin.rs before trusting the
   PIN with anything.
8. A real phone-first UI. What exists is a desktop test rig.
9. Android via `npm run tauri android init`. iOS needs a Mac.

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
