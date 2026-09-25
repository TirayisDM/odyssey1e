# odyssey1e - session handoff

**Written 2026-09-17, updated after initiative, the turn and the
encounter screen.**
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

And the world has places in it. A universe, a continent, a tavern and
a broom cupboard are the same kind of row at different depths - 033
stores `parent_id` and nothing else, and `locations.rs` works out depth
and path on the way to the screen. The DM panel builds the tree,
selecting a place says what is lying in it, and an encounter can say
where it happens.

UNHELD NO LONGER MEANS NOWHERE. Every drop control offers a place;
blank still means nowhere, but as a choice rather than the only
outcome. The "Lying nowhere" lost-and-found that 033 added for the
things dropped before there was anywhere to drop them is GONE, and not
because it emptied - the Objects tab shows every object in the game
with where it is, and "nowhere" is one of the answers, so a second list
for that one case was a second place to look. `loose_objects` went with
it: a registered command with no caller is a defect, not a spare.

AND THE SCREEN IS FIVE TABS. Play is the one a player lives in; Run,
World, Characters and Objects are DM tools, split by what they are
ABOUT rather than by when they get used. Before this the rig showed
every panel for every role in one column and stopped being navigable
somewhere around the DM screen.

  Play        the sheet, equipment, rolls
  Run         encounters, enrolment, the roster, challenges
  World       the map, and a scene: who is here, what is happening
              here, what is lying here
  Characters  PCs and NPCs, the statblock library, and the individuals
              made from it
  Objects     every object in the game and who is holding it, plus the
              catalogue

EDITING A CREATURE IS THE SHEET. Since 022 an NPC is a character in
every mechanical respect, so "open sheet" opens a goblin in the same
screen as a player. A second editor would be a second place for the
same rules to be wrong.

THINGS HAVE A SIZE AND CONTAINERS HAVE A REACH. Four questions gate
putting something in a container, and they are four different
questions:

  reach        can anybody touch both of these at once
  accepts      a purse takes coins, and that is not a coin  (032)
  size         nothing bigger than Tiny goes in, and that is Large (036)
  room         it would go in, but there is no space left    (032)

Reach is first because no answer to the other three matters if the
chest is in another building. Size is before room because it is the
objection a person reaches for and it gives the better sentence.

How full a container is shows as a bar and a number on every row, and
the number comes from the same sum that refuses the next thing - one
`slot_total`, so the gauge cannot read half empty while the container
says no.

AND THIS SWORD HAS ITS OWN MOVES. 050 lets a technique belong to an
OBJECT instead of to an item key, so the Mace of the Deep Song's ten
buttons can be given to one particular mace rather than to every mace
in the game. The object viewer lists what a weapon can do, the editor
adds and edits them on that object, and the character sheet ROLLS them
- the same `swing` path, matching on the object when a technique names
one and on the item key when it does not.

Removing one is a tombstone rather than a delete, because 050's rows
sit in the same table as 006's seeded catalogue: a per-object override
that hides a catalogue technique has to survive, and a row that is
simply gone would let the catalogue's version come back.

AND A FIGHT HAS AN ORDER. `encounter_actors.initiative` had been a
column nothing read since 011. 051 stores the round and whose turn it
is; `initiative.rs` derives the rest - the order, who is eligible, and
where the round rolls over - with 17 tests and no database in sight.
The order strip sits at the top of the Run tab with the current actor
lit, and the button names the creature it will hand the turn to rather
than saying "next".

Rolled live: Merchant 1 on 12, Goblin Scout on 7, Goblin Fighter 0004
on 2, in that order, with the round advancing when it wraps.

AND THE RUN TAB IS TWO SCREENS. A simple list of fights; opening one
narrows the list to it and offers View or Edit. View is what a DM reads
while running: the narrative, who is in it, what can be aimed at, and
three buttons that jump to the room in World, the people in Characters
or the things in Objects. Edit is everything that CHANGES the fight -
name, place, prose, status, enrolment, rolling and setting initiative,
hiding and removing.

They were one screen, and the forms had come to outnumber the facts.
A monster's attack buttons are grouped one line per weapon per mode
for the same reason - a goblin with a light hammer has fifteen of them,
and in one undivided row they read as fifteen unrelated verbs.

AND THE FIGHT KEEPS ITS OWN ACCOUNT. The attack buttons are on the
Run tab and their results were not: a DM swung, a goblin lost hit
points, and the only record of it was on the Play tab behind the whole
game's roll log. The View pane now carries what has happened in THIS
encounter, newest first, grouped by round - who swung, at whom, what
they rolled, hit or missed, and what it cost.

A separate read rather than a filter over the Play tab's list, because
that list is the newest fifty rolls in the GAME: a filter would quietly
empty itself the moment a fight scrolled off the end of it. The rolls
come back embedded under their action, which is what 012 made an action
own its rolls for.

AND A CREATURE'S ACTIONS ARE COUNTED. Nothing anywhere could say that a
character had swung three times in one round. 054 stamps the round onto
each action as it is written - by a trigger, for the reason 001 stamps
character_name: a round the client sends is a round the client can get
wrong, and a count built on it would be calmly false rather than
visibly broken. `spent.rs` does the counting, with tests.

The roster says "acted 3 times" and the order strip carries a x3, both
amber past one turn's worth. NOTHING REFUSES THE SECOND SWING, which is
051's decision unchanged: Extra Attack, haste, an action surge and a DM
simply allowing it are all ordinary, and the ask was to be able to SEE
it rather than to stop it.

**423 tests, zero warnings.** `cd src-tauri && cargo test`.

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
026 objects get identity - `character_items` becomes `objects` with a
    surrogate id; a sword becomes a particular sword
027 the catalogue - the SRD weapon and armour tables, and the two
    columns versatile needed; 49 seed rows
028 statblock proficiency - npcs gets the two arrays characters has
    had since 008, and instantiate_npc stops faking them
029 level is hit dice - a goblin is 2d6, so a goblin is level 2; hit
    points stop being a magic number
030 objects outlive their holder - deleting a creature drops its gear
    instead of destroying it
031 entities and holders - the truss 026 said to wait for; objects
    point at a holder, not at a character
032 containers - a spell book, a treasure chest and a coin purse
033 locations - somewhere to be; a location is an entity, and depth is
    the whole hierarchy
034 encounters in places - a fight happens somewhere, and cancelled
    becomes its own word
035 characters stand somewhere - where a person is, which is a fact and
    not a derivation
036 things have a size - a greatsword does not go in a coin purse, and
    the ladder is the one creatures already use
037 loose objects are everyone's - finishes 031's policies, which
    assumed the only alternative to being held was being nowhere
038 one depth cap - three walks up the same chain had three limits
039 platinum - the fifth coin, so the 5e ladder is whole
040 shops - what a merchant stocks and what it charges
041 trade - one exchange that lands whole, in both directions
042 the armoury - twenty weapons the SRD leaves out
043 special attacks - every weapon gets at least three
044 two misfiled sizes - trident and morningstar, filed by name rather
    than by what they are
045 weapons take room - slots off the size ladder, doubling each step
046 a quiver holds ten - the arrow was 0.05 of a slot, so a quiver held
    two hundred
047 armour and gear take room - and two defects found on the way:
    platinum too big for a coin purse, ring mail filed as medium
048 review the armoury - every weapon read back against its own ladder;
    the net gets a weighted rim and therefore three attacks
049 this one is different - everything about one object, editable on
    that object
050 this sword has its own moves - a technique can belong to an OBJECT
    rather than to an item key, and removal is a tombstone
051 whose turn it is - the round and the current actor; the order
    itself is derived
052 a challenge can be about a thing - an object becomes targetable by
    having a challenge attached, not by being a third kind of target
053 an encounter has something to say - narrative prose, and retiring
    an encounter instead of deleting one
054 an action remembers its round - stamped by a trigger, because a
    timestamp cannot say which round a swing belonged to

All applied. Files in `supabase/migrations/`. **Read the comments** -
each one carries why it exists, and 003 and 004 are fixes for my own
mistakes with the reasoning written out.

**007 and 008 are read by `equipment.rs` now.** 007
replaced `techniques.weapon` - free text holding a display name like
'light hammer (thrown)' - with `item_key` plus `mode`. A display name is
not an identifier, and that one string was carrying two facts: which
item, and which attack mode. The light hammer has different technique
lists for melee and thrown, 7 and 6.

**029 made a monster's level its HIT DICE.** The Monster Manual always
wrote it that way - `Hit Points 7 (2d6)` - and 029 stops treating the
bracket as trivia. `npcs.hp_max` was a magic number: 7, because the
book says 7. Nothing could check it, nothing could move it, and a DM
who wanted a tougher goblin had to write a second statblock.

The rule is `vitality.rs`: `floor(level * (die + 1) / 2) + level *
con_mod`, with the die coming from SIZE - d4 d6 d8 d10 d12 d20 for the
six categories, which is why the book never states it separately. Its
tests check five printed Monster Manual lines from d6 to d20, and those
are the most valuable tests in the repo because anyone holding the book
can falsify them:

    Goblin     2d6,  CON 10    7     Orc    2d8,  CON 16   15
    Bugbear    5d8,  CON 13   27     Ogre   7d10, CON 16   59
    Tarrasque  33d20, CON 30  676

Two things that are easy to get wrong from memory and are pinned by
their own tests: Constitution applies PER DIE (the ogre's +21 is +3
across seven), and the TOTAL rounds down rather than each die (7d10 is
38.5 to 38, not seven lots of 5.5 to 35).

The goblin's hit points did not move. It was level 1 with a stated 7
and is now level 2, and 2d6 averages to exactly 7 - the number is the
same and has stopped being arbitrary, which is the whole point.

**`set_actor_level` moves it on the INDIVIDUAL**, not the statblock.
Levelling Crumbs makes Crumbs tougher; the shared goblin is untouched.
022 settled that, and this is the first command that would have been
ambiguous before it. Wounds survive a level change and that falls out
of 013 rather than being arranged - hit points are a log, so raising
the maximum by seven leaves a creature at 3 of 7 sitting at 10 of 14,
still down by four.

The proficiency override is CLEARED when a DM levels something by hand,
and that is a decision rather than a side effect. A statblock states
its bonus because the book rates a monster by CHALLENGE, not hit dice -
an ogre is 7 dice and CR 2, so the book says +2 where level derives +3.
That answer is about the monster the book printed; a creature a DM has
hand-levelled is not that monster any more.

**What does NOT ripple, and do not assume it does.** Skills would ride
on the proficiency bonus for free, except `npcs` carries no skill
proficiencies at all - so there is nothing riding on it yet, and giving
statblocks skills is its own piece of work. Special abilities are not a
ripple, they are a missing subsystem: there is no table, so a goblin's
Nimble Escape does not exist anywhere in this schema and levelling one
cannot scale something the database has never heard of.

A statblock with no `size` cannot derive a die, and that is reported
rather than guessed - guessing d8 would quietly hand every sizeless
statblock a medium creature's hit points. `Goblin Fighter` (`0000A1`)
is the live example: written through the DM panel before the form had a
size field, so it keeps its stated 23 and refuses to be levelled until
somebody gives it one.

**028 is the bug the inventory system exposed.** `characters` has had
weapon_profs and armor_profs since 008 and `npcs` never got them, so
`instantiate_npc` built a monster with empty arrays and covered for it
by stamping `coalesce(x.proficient_override, true)` on every kit row.
That reads as "a monster is proficient with its gear". What it says is
"proficient with exactly what came out of npc_items and nothing else,
ever" - and it was invisible until there was a way to hand a monster
something. One sickle later:

    Runt   · Scimitar  1d20 +4   DEX +2, PB +2   the stamped override
    Crumbs · Sickle    1d20 -1   STR -1, no PB   derived, against {}

Same species, same round, and the only difference was which table the
weapon arrived through. 028 gives `npcs` the arrays, copies them on
instantiate, and drops the coalesce, so `is_proficient` decides it for
a goblin exactly as it decides it for Rodnar - unchanged, one rule.

The goblin is `{sim}` plus an explicit override on its scimitar, NOT
`{sim,mar}`. A goblin knows the blade it carries; it is not a
martial-weapon user in general, and promoting the species would hand it
a greatsword and a longbow it has never held. Verified through the live
function and rolled back: handaxe and sickle proficient by derivation,
scimitar by override, and a greataxe correctly NOT - which is the
control proving the rule still discriminates.

**026 is the one to read before touching inventory.** `character_items`
was a junction keyed `(character_id, item_key)`, and that key was the
limit: two shortswords could not differ, nothing could be named, and
nothing could exist unheld. It is now `objects` - the individual, with
who holds it as one of its facts. Stacks survive on purpose (seven
rations are one object with quantity 7), a named thing is always
quantity 1, and `character_id` is nullable so a dropped thing can exist
with no holder.

What 026 is NOT is the `entities` component work. A holder is a
character and only a character, with a real FK. When a location or
another object can hold something, THAT is when `entities` is earned -
doing it now with a nullable character_id beside a nullable location_id
under an XOR is exactly the shape 022 and 023 removed.

**027 filled the catalogue**, which had 15 rows because it had only ever
grown to satisfy whoever asked last. Now 37 weapons and 13 armours -
the SRD tables. Three honest gaps are written into its header:
versatile stores its second die in new columns that NOTHING READS yet
(a Versatile mode means widening `techniques_mode_check` and teaching
the resolver, which is a rule change with its own tests); heavy armour's
Strength requirement has no column because no rule reads it; and the
blowgun is absent because a flat 1 damage is not a die and
`damage_denomination` is checked `>= 2`.

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

**A FAULT WEARING THE COSTUME OF AN ORDINARY ANSWER. THREE TIMES NOW.**
This is the shape of defect this codebase keeps producing, and it is
worth naming as a class rather than as three incidents:

  016  an RLS-blocked UPDATE succeeded and did nothing
  031  a renamed parameter still compiled, so every inventory read came
       back empty and PRESENTED AS A WRITE FAILURE
  036  a `\n` in a select emptied the entire object manager, and the
       empty list rendered as "no objects yet"

Every one of them produced a plausible, calm, wrong answer instead of
an error. None was caught by a test, because none of them is wrong in
isolation - the failure is that the failure path and the ordinary path
render identically.

The general fix, applied twice so far: MAKE THE TWO PATHS DIFFERENT.
`supabase::rest_get` now refuses a query parameter holding a control
character before it sends anything, because no select, filter or order
has a use for a newline - one is always a typo. And the object and
character managers use `tryCall` and say what went wrong rather than
painting an empty state over it.

**THREE SHAPES OF "AN OBJECT" REACH THE FRONTEND AND THEY DO NOT NAME
THE ITEM THE SAME WAY.**

  objects::Stack     item_key, the bare key
  holders::Located   item_key, the bare key
  equipment::Owned   item, the whole catalogue row

Reading `o.item_key` off an `Owned` gives undefined, so a contained
object with no given NAME rendered as a blank row - and a named one
rendered fine, which made it look like one missing item rather than a
broken field. Every frontend function taking "an object" has to know
which of the three it holds, and getting it wrong produces a blank
rather than an error. Unifying them is a contained refactor nobody has
done.

**A RULE ENFORCED IN ONE COMMAND IS NOT ENFORCED.** `put_in_container`
asked the three capacity questions and was the only thing that did,
because it was the only command that MOVED anything. `clone_object`
copies a row into the same holder and `edit_object` sets a quantity
outright - neither moves anything, so neither ever reached the check,
and both could put a twenty-sixth coin in a purse that holds
twenty-five. Clone is a button on every row and quantity is a box in
every editor; neither is exotic.

The gate is now one function every way in calls. When adding a rule,
the question is not "does the obvious command check it" but "what else
writes to this table".

**A BUNDLED ROOT STORE BREAKS ON ANY MACHINE RUNNING ANTIVIRUS, AND THE
ERROR POINTS THE WRONG WAY.** `reqwest`'s `rustls-tls` compiles a copy
of Mozilla's root list into the binary and ignores the operating system
store. Antivirus that inspects HTTPS - Avast's Web Shield here - stands
in the middle with its own certificate, signed by a root it installs
into WINDOWS. Every other program reads that store and carries on; this
one did not, and the handshake failed.

It reports as `error sending request for url` WITH NO HTTP STATUS. No
401, no 403, nothing naming a certificate - it reads exactly like the
network being down, and it was called transient twice before it was
understood. What gave it away was the asymmetry: curl and PowerShell
reached the same URL from the same shell and got an answer while the
app could not connect at all. Two programs, one machine, one network,
different results only happens when they disagree about who to trust.

`rustls-tls-native-roots` is the fix and it is one line. It would have
hit players as hard as it hit this laptop, and looked like a broken app
to them and a working one to everybody else.

**A RENAMED PARAMETER THAT STILL COMPILES IS THE EXPENSIVE KIND.** 031
moved objects onto entities and renamed `load_loadout`'s second
argument from `character_id` to `holder_id`. One call site kept passing
`character_id`, and it compiled, because both are `String`. Every
inventory then came back empty - PostgREST was being asked for objects
held by a character id, which nothing ever is.

It presented as a WRITE failure. Giving a dagger to a goblin worked
three times and the panel said "carrying nothing" each time, so three
successful writes read as three failures. A screenshot of the panel
beside the row in the database is what settled it.

Two of the three callers were right, which is why a sheet's loadout
kept working while the full inventory emptied - and why nobody noticed
for a day. When a rename is only a rename to the compiler, grep the
call sites. (031, fixed here)

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
hit point and clears the tally, three either way settles it, and being
hit while down costs a failure and two on a crit.

And two more that ARE 5e and are now implemented properly, after a
goblin was found at -35 of 7 while still rolling saves:

**Hit points are floored at zero.** 5e has no negative hit points at
all - that is the 3.5e rule, where you sank to -10. Damage past zero
leaves you AT zero and the excess is discarded, except for the check
below which weighs it first.

The floor is applied where hit points are READ, not where damage is
written, and that distinction is load-bearing. 013 made hit points a
log and the log is evidence: a goblin that took fourteen took fourteen,
and clamping the event would record a blow that never landed. Both
places that sum the log now floor the total - `load_targets` and
`load_actor_vitals` - and nothing else changed.

**Overflow death is checked on every blow, not just the one that drops
you.** `massive_damage_kills` was always right; `lib.rs` only called it
behind `was.is_conscious()`, so it ran once and never again. That is
how -35 happened. The book checks it whenever damage lands, and at zero
there is nothing left to subtract - the whole blow is overflow and the
raw damage is weighed against the maximum.

`death::overflow` is what makes that one expression rather than two
branches: it subtracts the FLOORED current hit points, so a creature at
five taking twenty spills fifteen and a creature already at zero taking
twenty spills all twenty. The second case is the book's rule for damage
taken while down, arrived at without a second code path.

The evidence it mattered, from the live database: Crumbs had taken 42
past a maximum of 7, and two of those blows individually met or
exceeded that maximum. Either should have killed it outright. Runt is
the control - 9 summed, worst single hit 5, so it correctly sits at
zero and goes on saving. Past rolls are not reconciled; the log is
history, the same reasoning 025 used for names.

One more, in `resolution.rs`: a natural 20 does NOT carry an ability
check, only an attack. That is rules as written, and
`TargetKind::auto_decides` is the single line to change if the campaign
ever disagrees.

---

## Architecture decisions that should hold

**EIGHT LIBRARIES. THE FRAMEWORK, NOT A COMPRESSION.**

Not three tables that everything squeezes into - three primary
libraries for the nouns of the game world, plus five more for the
things that are different in kind, each with its own family of tables
underneath. It answers "where does this go?" before the question turns
into an argument.

    Game         games, game_members, profiles
    Characters   characters, character_abilities, character_skills,
                 character_items, character_dice, npcs, encounter_actors
    Objects      items, npc_items, dice_sets, dice_faces
    Locations    encounters, encounter_challenges
    Rules        skills, techniques, spells
    Resources    narrative_lines, skill_prompts
    Ledger       rolls, hp_events, actions
    Actions      - empty -

RESOURCES MEANS ASSETS: prose, art, portraits, prompts. Not
expendables. Hit points and charges are mechanics and live with the
things that have them.

THE EMPTY AND THIN SHELVES ARE THE ROADMAP, which is the part worth
having. Actions is empty because `actions` is a LEDGER row - a swing
that happened, owning its rolls - and no action economy exists: no
action, bonus, reaction or movement, because none of that can exist
before turns do. Locations is thin because `encounters` is standing in
for place; there is no room, no map, no floor to drop a sword on. And
Objects has a template with no instance, which is the gap below.

ONE SHAPE CUTS ACROSS ALL OF THEM and is not a library: nullable
`game_id` with two partial unique indexes - a global row and a campaign
override sharing a key. items, npcs, skills, narrative_lines,
dice_sets, spells and techniques all use it. It is a TENANCY pattern,
and filing it as a place in the taxonomy would be a mistake.

**COMPONENTS FOR SHARED MECHANICS. DECIDED, NOT YET EARNED.**

A goblin, a door and an inn all have hit points, and it is the same
mechanic each time - a number that goes down. The right shape is a
component table joining whatever has one, rather than six HP columns
copied onto every table that might be damageable.

THE VERSION THAT FAILS HERE is a polymorphic owner: `owner_id` plus
`owner_kind`, or nullable character_id/object_id under an XOR check.
That shape has been built twice in this schema and removed twice in one
week - hp_events lost its XOR in 023, encounter_actors lost its in 022 -
because it branches at every read and cannot carry a foreign key.

THE VERSION THAT WORKS needs a real id space, which is the second beam
the truss spans between:

    entities(id, kind, game_id)
    characters.entity_id -> entities     a real FK
    objects.entity_id    -> entities     a real FK
    locations.entity_id  -> entities     a real FK
    hit_points(entity_id -> entities, ...)
    ammunition(entity_id -> entities, ...)

AC IS NOT ONE OF THESE, and it is the warning case. A character's AC is
COMPUTED from what they wear; a door's is STATED. That is two
derivations, not one mechanic with two owners, and 022 already settled
it with ac_mode flat/default.

WHAT EARNS THE BUILD is the first damageable object. Today only
characters have hit points, so the truss would span a gap with nothing
on the far side, at the cost of migrating HP and the death counters off
`characters` - the hottest table in the schema, read on every roll.
`characters` is 26 columns and carrying HP, death saves, exhaustion,
inspiration, size and both proficiency arrays, so it is the table most
likely to want this first. Queued behind a need, not dismissed.

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

## Combat - BUILT, and the turn with it (051)

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

**THE TURN IS BUILT, AND IT GATES NOTHING.** That is the decision, not
an unfinished edge. The app shows the order and refuses nothing: anyone
can act at any time, and the strip says whose turn it was meant to be.
A DM overrules the order constantly - a held action, a surprise round,
somebody who arrived late - and software that enforces it turns every
one of those into an argument with the tool.

`initiative.rs` derives the order from numbers already on the actors:
initiative, then DEX, then name, then id - two goblins can share a
name, and the id is the only tiebreak guaranteed to differ - so the
same roster always comes back in the same sequence. A creature
that has not rolled, one that is hidden and one that is dead are all
OUT of the order and still ON the screen saying why - taking them off
would answer "where did the goblin go" with silence.

**NPCs roll on enrolment, players roll their own.** Nobody is at the
table to roll for a goblin. Both the roll and the location stamp are
best effort: if either fails the enrolment still stands, because a
goblin sitting at "not rolled" is recoverable and an enrolment that
silently did not happen is not.

**The DM side is built too.** Encounters, actors, challenges and
statblocks all have screens - see "the Run tab is two screens" above.

WHAT IS STILL MISSING:

**The action economy.** 5e splits a turn into an action, a bonus
action, a reaction and free interactions, and nothing in the schema
knows which a technique costs. 054 counts ACTIONS, which is one honest
number; four would mean inventing a classification for 193 catalogue
rows. `techniques.action_cost` is the migration, and the seed data is
the work.

**Death saves still happen on a button.** 015 said "until initiative
arrives". It has arrived, and hanging the save on the creature's own
turn is now the only place the build departs from the rule as written.

**Attacking a THING.** 052 makes an object targetable by hanging a
challenge on it, and a challenge is a DC - a lock that can be picked, a
door that can be forced. A door with an AC and hit points is a
different claim and was deliberately not built: it wants vitals on
objects, hp_events against a subject that is not a creature, and a rule
for what a broken thing becomes. 052's header states what it refused to
decide. Nothing in the live game has used it yet - zero challenges
point at an object - so it is built and unproven.

---

## Inventory - BUILT (026, 027, objects.rs, commands/inventory.rs)

**There was no way to acquire an item until now, and there had been one
by accident.** `set_item_equipped` upserted on `(character_id,
item_key)`, and an upsert on a pair that is not there INSERTS - so the
way to give Rodnar a longsword was to equip a longsword he did not have.
Nobody designed that path; it was a side effect of the write. 026 closed
it by making the command take an object id, and `give_item` is the path
that replaces it.

The commands are `list_catalogue`, `give_item`, `drop_object`,
`take_object`, `rename_object`, `destroy_object`, in
`src/commands/inventory.rs`. Every decision they make is in
`src/objects.rs` and tested there; the command files hold no rules,
per the directory's own line.

**One command arms both sides.** `give_item` never asks whether the
holder is a player or a monster, because since 022 there is nothing to
ask - a goblin is a `characters` row. What stops a player arming the
goblin is the RLS policy, not the command and not the panel. A rule
enforced in two places is a rule that will disagree with itself.

The four rules worth naming, all in `objects.rs`:

- **A named thing never merges**, in either direction. It does not join
  a plain stack, because then its name would be a fact about all seven;
  and a plain thing does not join a named one, because "Runt's Axe x2"
  is not a sentence. This is `objects_stack_idx` stated as a rule rather
  than as an index.
- **A stack cannot be named.** Split one off first - the question has no
  answer otherwise, and the error says so.
- **Drop and destroy are different events.** A dropped thing stays in
  the campaign with `character_id` NULL and can be picked back up; a
  destroyed one is gone. Only destroy asks first.
- **A partial drop splits.** The held row keeps the remainder and a new
  unheld row carries what left, because the seven rations were never
  seven objects.

**030 stopped a delete from eating inventory.** 008 made
`character_items.character_id` NOT NULL with ON DELETE CASCADE, and
that was correct: an object with no holder could not be represented, so
there was nowhere for an orphan to go. 026 removed the premise and left
the conclusion - character_id became nullable, NULL came to mean nobody
is carrying it, and the cascade went on destroying things anyway.
Clearing some stale test goblins would have taken a scimitar, a handaxe
and a sickle with them, not because anyone decided gear should be
destroyed but because a rule written for a different schema was still
running.

SET NULL now. Deleting a creature drops what it was carrying. This puts
objects in the same category `rolls` and `actions` were already in -
some things outlive the row that pointed at them - while hp_events,
abilities, skills, dice and actors go on cascading, because every one
of those is a fact ABOUT a creature and means nothing without it. An
object is not: it is a thing in the world that was in someone's hands.

**The trigger is the half that is easy to miss.** An object that
becomes unheld must stop being equipped, and `drop_object` said so in
Rust - but a CASCADE runs inside Postgres where no command does. Without
`unheld_is_unequipped`, deleting a character would leave a breastplate
on the floor still flagged equipped, and `take_object` would put it
straight onto whoever picked it up: equipped, unchecked, bypassing the
one-armor rule. The rule moved to the database rather than being copied
there, and `drop_object` no longer restates it.

Verified against live data and rolled back: deleting Crumbs left all
three of its weapons unheld, unequipped and still in the campaign, its
actor row correctly gone, and the three rolls that named it still
readable with character_id set to null.

**A PLAYER CANNOT PICK UP LOOT, and that is now worth knowing.** The
`objects: holder or dm` policies reach a campaign either through
`is_game_dm` or through the holder's owner_uid - and an unheld object
has no holder, so only the DM can touch one. That was invisible while
unheld objects barely existed; 030 makes them ordinary. Nothing here
changed a policy, because widening access control is a decision rather
than a consequence. `take_object` is DM-only until it is made otherwise.

## Containers - BUILT to a minimum working state (031, 032)

**031 IS THE TRUSS 026 SAID TO WAIT FOR.** Its header set the
condition: "When a location or another object can hold something, that
is the moment `entities` is earned - and doing it now with a nullable
character_id beside a nullable location_id under an XOR is precisely
the shape 022 and 023 removed." A container can hold something, so the
moment arrived.

`entities(id, game_id, kind)` is a shared id space over the things that
can HOLD. A character gets one. A chest gets one. A room will get one.
An arrow does not, because nothing goes inside an arrow.

**THE PRIOR ART IS DAVE'S OWN.** odyssey-engine's `container.rs` has
carried `HolderType { Actor, Tile, Container }` and `Placement {
item_uid, holder_type, holder_id }` since long before this port. 031 is
that, written as a schema. The tag vocabulary and the slot arithmetic
came across intact too - `coin_purse()` there is 5 slots restricted to
`["coin"]`, and a coin is 0.2 of a slot because 5 slots is 25 coins.

`objects.character_id` is GONE. An object now carries:

    holder_id   WHERE IT IS - one column, one FK. NULL is nowhere.
    entity_id   WHAT IT IS, when it can hold. NULL on an arrow.

A chest carries both: it is somewhere, and things are inside it. Two
columns answering "where is this" would have been two sources of truth,
and 028 is what that looks like when the second one rots.

**A CONTAINER IS AN OBJECT**, and everything follows from that. A coin
purse is carried, dropped, stolen and put inside a backpack, so it is
not a table of its own. `container_gets_an_entity` gives it its holder
identity on the way in, mirroring the character trigger.

**Where each rule lives, and why they are not in the same place:**

- **Acceptance** (does a purse take a sword) is RUST, in
  `containers.rs`, tested. It needs `items.accepts` on one row and
  `items.content_tags` on another, and a CHECK sees neither - the same
  reasoning 008 gave for the one-armor rule.
- **Cycles** are POSTGRES, in `no_container_cycles`. That is integrity,
  not rules: a cycle does not make an answer wrong, it makes
  `holder_character` walk until its guard trips, and every policy on
  `objects` calls that function.
- **Ownership through nesting** is `holder_character(uuid)`, which
  walks up the chain. A purse inside a backpack carried by Rodnar is
  Rodnar's, and a policy that looked one level up would say it is
  nobody's. Depth-capped at eight.

**A SWORD IN A CHEST IS NOT EQUIPPED.** 030's trigger could only ask
whether a holder was NULL. It asks a better question now - is the
holder a CHARACTER - so putting a breastplate in a backpack takes it
off, and `set_item_equipped` refuses anything that is inside something.

**Verified live and rolled back:** Character1 carries a named purse
holding 15gp and 8sp at 4.6 of 5 slots; nesting the purse inside the
backpack still resolves the coins to Character1; a container refuses to
go inside itself both directly and through a chain.

**Seeded:** backpack, coin purse, treasure chest, spell book, quiver,
plus arrows and three coin rows. COINS ARE A DUMMY, as asked - three
`loot` rows tagged `coin` so the purse has something to accept and
everything else to refuse. `items.kind` has no 'currency' value and 032
deliberately does not add one, because that belongs with the subsystem
that needs it. Currency is next.

**NOT CARRIED OVER from odyssey-engine, all deliberate:** `ItemSize`
and a size limit (so a greatsword fails on a purse for a second and
better reason than its tags), open/closed and locked state, and a
nesting depth separate from the cycle guard. Capacity is slots only.

**Unheld still means nowhere.** A dropped object has no location,
because Locations is a stub - no room, no floor, no container. It is
loot in limbo, and it is still worth having: it is what a drop, a chest
and a shop all need, and the alternative was deleting things people let
go of.

The DM's actor view shows a monster's WHOLE inventory now, not its
loadout. A loadout is what is equipped, and a DM who had just handed
over a longsword would have watched the panel repaint without it -
`give_item` arms nobody, deliberately.

**NOT `inventoryRow`.** The actor view builds its own kit row, because
`inventoryRow` offers techniques and reads them off `state.sheet` - the
signed-in player's sheet - so rendering a goblin's axe with it would
have offered Rodnar's Heavy Smash to the goblin. That is the shape of
bug this project keeps finding: two layers that each look right alone.

## Acquisition - SPECIFIED, rules built, plumbing not (acquire.rs)

Dave's five options for how an object changes hands. They are FIVE
NAMES FOR TWO MECHANISMS: one event - an object changes holder - gated
either by a roll or by consent.

    Take         contested   overt
    Pick Pocket  contested   covert
    Buy          consented   by price
    Sell         consented   by price, reversed
    Give         consented   by the owner

`give_item` and `take_object` are already the transfer. Nothing in
`acquire.rs` moves anything; it answers what happened and something
else writes it down - the same division `attack.rs` and `swing` have.

**THE TAKE RULE, settled 2026-09-22 and tested:**

- A dead or unconscious holder gets nothing. No notice, no struggle.
  Looting a body is not a contest.
- Otherwise the holder rolls WISDOM TO NOTICE - and that save does NOT
  stop the take. It only decides whether they see it happening. Miss it
  and the object is simply gone.
- Notice it and it becomes DEXTERITY AGAINST DEXTERITY. A tie leaves
  the situation as it was, so the holder keeps it.

Notice and success being separate questions is the whole point. Folding
them into one roll would lose the case that makes the feature worth
having: taking something cleanly from somebody who never knew.

Pick Pocket is the thinner reading, marked as such in the file: one
roll decides both, and being noticed ends it. No wrestling a purse out
of a hand that has already closed on it.

**WHAT IS NOT BUILT, and what each one is blocked on:**

- **The plumbing for Take.** A player-initiated take cannot be a REST
  write - the `objects` policies reach a campaign through `is_game_dm`
  or the holder's owner_uid, and a player has no claim on someone
  else's gear. It needs a SECURITY DEFINER function running the contest
  and the transfer together, the shape `write_action` uses. That is the
  next piece and nothing blocks it.
- **Traps.** `Step::SpringsTrap` exists and nothing can ever return it,
  because there is no trap subsystem - no table, no column. The step is
  in the enum because the ORDER was specified and is worth not losing:
  a trap on the body or the container answers before anyone notices
  anything.
- **Give to a LOCATION.** Give to a character works today. Locations is
  still the stub 026 complained about, so there is nowhere to put a
  thing down.
- **Buy and Sell.** Not "rules to come later" - a subsystem first.
  `items.price` and `denom` exist and nothing holds coins: no wallet on
  `characters`, no currency anywhere.

**One default I chose rather than was given:** the Wisdom save is
against 10 + the taker's Dexterity modifier. The taker makes no roll to
Take - that was the spec, and it is what separates a Take from a Pick
Pocket - so the difficulty has to come from somewhere, and a DM-stated
DC overrides it the way `add_challenge` states every other one.

## Locations - BUILT (033, 034, locations.rs, commands/locations.rs)

**A LOCATION IS AN ENTITY.** That is the whole design. 031 built
`entities` as a shared id space over things that can HOLD, and its
header said a room would get one; 033 is that. `objects.holder_id`
already pointed at an entity, so a sword rests on a floor with NO
CHANGE TO THE OBJECTS SCHEMA AT ALL.

**DEPTH, NOT TIERS.** `parent_id` is a nullable self-reference and that
is the entire hierarchy - a universe, a continent, a tavern and a broom
cupboard are the same kind of row at different depths. No level column
and no path column; `locations.rs::arrange` derives both.

The prior art argues FOR this rather than against it. OdysseyAIRPG's
scene schema materialises `hierarchy.level` and `hierarchy.path` with
the convention "0=region, 1=settlement, 2=district, 3=building", and its
56 live scenes already disagree with that comment. Deriving is the
lesson from it, not a refinement of it.

`kind` is the MEDIUM, not the rank: structure, outdoors, underground,
aquatic, aerial, vehicle - the axis that original's Config.js documents.
Depth says how far in you are, kind says what sort of place it is, so a
ship is a vehicle that contains rooms. CHECKED, because that field
drifted there: the scene type list is six values and the live data holds
`kingdom` and `village`, a FACTION type from campaign.js that leaked
into a place type with nothing to stop it.

**ZONES DO NOT EXIST HERE, deliberately.** The original needed them
because a scene was heavyweight, so a room inside a building had to be
a lesser thing with its own shape and a `parentZone` special case. A
location is cheap, so the Watch Room is a location whose parent is The
Guardhouse. One concept instead of two.

**What it ended:** unheld meaning nowhere. Both 026 and 032 accepted
that a dropped object had no location because the alternative was
deleting what people let go of - and a handaxe dropped on the 21st then
sat invisible for a day, because nothing renders nowhere. Every drop
control now offers a place. The lost-and-found that shipped with 033
is already retired - superseded rather than emptied, which is better:
the Objects tab lists every object in the game with where it is, and
"nowhere" is one of the answers.

**Guards, all verified firing:** a place inside itself, a cycle through
any chain, a parent in another game, and deleting a place that still
contains one. `parent_id` is ON DELETE RESTRICT on purpose - deleting a
continent should not silently delete its settlements and spill every
object in them.

**What 033 exposed:** a player could not pick anything up. Every write
policy on `objects` asks `holder_character(holder_id)` for a character
the caller owns, and a loose object has no holder, so only the DM could
ever take one. Nothing rested anywhere before, so nobody met it.
`holder_character` needed no change - a location's entity matches
neither branch of its walk, so an axe on the floor is located and
unowned, which is right. The update policy gained a third path instead:
loose things are anyone's to take. DELIBERATELY GENEROUS, and the thing
access control narrows.

**Not here, deliberately:** travel and `scope`, access, ownership,
property, coordinates. The original defined all of them up front and
`location.position` is still null years later.

---

## The three free rules - BUILT (carry.rs)

Three things the schema could always express and nothing enforced. Each
fact had been in the catalogue since 008 - the `attuned` flag, the `two`
property, the `weight` column - with no rule reading it. That is the
quiet kind of gap: it does not fail, it permits something the rulebook
forbids.

They live in `carry.rs` rather than `equipment.rs` because that file
answers what a character can DO with what they hold and is 862
production lines against an 800 ceiling. These ask what a BODY can bear.

**ATTUNEMENT, capped at three.** There was no way to attune at all -
`objects.attuned` has existed since 008, is read onto every sheet, and
nothing in the command surface ever wrote it. The Ember has been attuned
since the seed and could not have been released. So the rule and the
only door to it arrived together in `set_item_attuned`.

Counted across everything a character ultimately holds, not just what is
equipped: a wand at the bottom of a backpack is still one of your three.
Unlike equipping, attunement does not require a thing be in hand.

**HANDS, and it is the general rule rather than the case asked for.** A
budget of two, with a two-handed weapon costing both and a shield one.
That covers the greatsword-and-shield case and three nobody had written
down: two greatswords, three weapons, and a two-hander sharing a grip.
Armour costs nothing, which is why this cannot be a count of equipped
rows.

IT IS A TIGHTENING. 008 left `equipped` unconstrained on purpose, noting
several weapons may be held at once - right while nothing modelled
hands, and a licence rather than a rule.

**CRUMBS IS ALREADY ILLEGAL and that is left alone deliberately.** It
holds a handaxe, a scimitar and a sickle - three hands, from the sickle
handed to it while testing 028. Unequipping is not checked, so the state
is self-correcting; equipping a fourth thing, or re-equipping the
sickle, is now refused with the arithmetic shown. Fixing the row by hand
would hide the one live demonstration that the rule works.

**ENCUMBRANCE, reported and not refused.** Fifteen pounds a point of
Strength, multiplied by size - a Large creature carries twice a Medium
one, which is why an ogre shifts a portcullis a halfling of the same
Strength cannot. Tiers at 5x, 10x and 15x.

The base rule forbids going over capacity outright and the variant lets
you and slows you down. They are different games, so the engine says
which side of each line somebody is on and leaves the consequence to a
rule the campaign picks.

It is a COMMAND, not a sheet field. `load_sheet` runs on every roll and
this walks everything held at any depth to sum it - a backpack weighs
what it weighs plus what is in it. That is the right cost for an
inventory screen and the wrong one ahead of a d20.

Live, on Character1: 113 lb of 180, STR 12 - encumbered, past the 60
that 5x buys.

## Money and shops - BUILT (039, 040, currency.rs, store.rs)

**COINS ARE ITEMS**, which 032 stumbled into by seeding a dummy:
HOPPER's `is_coin()` is `has_content_tag("coin")`, so the placeholder
was already the right shape and only needed rules written against it. A
coin is carried, dropped, stolen, weighed and put in a purse like
anything else, because it IS anything else.

**The ladder is 5e's**: cp 1, sp 10, gp 100, pp 1,000. NO ELECTRUM, and
there is a better reason than taste - `make_change` is greedy, and
greedy change is only optimal when each denomination divides the next.
1/10/100/1000 does; inserting 50 breaks it, and a till could then hold
enough to make change and fail to find it. `cp_per` still knows electrum
so a price written that way resolves, and no `coin_ep` row exists.

**The algorithm is HOPPER's**, ported unchanged from
odyssey-engine's `merchant.rs`: pay smallest-denomination-first (you
spend coppers before breaking gold, which also leaves the purse able to
afford the NEXT thing), change largest-first, overpayment recorded
rather than the sale refused.

HELD AND WORTH ARE DIFFERENT QUESTIONS, which live data surfaced: 18
gold and 8 silver is 1880 copper, and `format_cp` renders that as "1 pp,
8 gp, 8 sp" - right, and naming a coin nobody carries. Both exist.

**A SHOP IS A CHARACTER**, and almost all of it already existed - the
keeper is a characters row (022), the stock a container (032), the till
coins (039), standing in a place (033), with `move_into` already
transferring under a reach check. 040 adds two columns and one
transaction.

    markup       NULL MEANS NOT A MERCHANT. Nullable rather than
                 defaulting to 1, or every goblin is a shop.
    disposition  allied 0.80 .. hostile 1.25, sworn_enemy refuses.
                 NULL reads as neutral.

Prices are `base x condition x markup x disposition`, floored at a
copper, and merchants pay **0.40** of what they charge - HOPPER's
number, kept over 5e's informal half. CONDITION IS A PARAMETER NOTHING
FEEDS: no column records wear, so every live caller passes 1.0. The
seam is real and the subsystem is honestly absent, the same way
`acquire.rs` takes `trapped` before traps exist.

Haggling is **Persuasion against Insight**, the difference becoming the
percentage, capped at 25 either way - without a cap a lucky roll halves
a suit of plate, which is worse than losing. A tie moves nothing, the
same rule every contest here uses.

**THE PURCHASE IS A POSTGRES FUNCTION**, because buying is three moves -
goods across, coins out, change back - and over REST that is three round
trips with no transaction, where the gap between the first and second is
a free item. 012 settled that shape for actions and this is the same
claim about goods and money.

`buy_object` TRUSTS ITS CALLER ABOUT THE PRICE. store.rs holds those
rules and duplicating them in SQL would be the two-places problem this
repo keeps meeting - which is safe while the DM runs the table and is
NOT safe the day a player client calls it directly. The function's own
comment carries that warning.

Verified live and rolled back: Halla the Outfitter, markup 1.0 and warm,
sold Character1 a longsword. Buyer 18 gold and 8 silver became 3 and 15;
the keeper took 15 gold and gave 7 silver from a till of 40. Both sides
balance to the copper and the sword changed hands.

Buy works. SELL DOES NOT EXIST YET - the price is quoted (`shop_pays`)
and nothing acts on it.

## Trade - BUILT (041, store.rs, commands/store.rs, the Trade tab)

**ONE EXCHANGE, IN BOTH DIRECTIONS.** 040's `buy_object` was the
asymmetric special case and 041 replaced it. It had a buyer and a
merchant baked into its signature, goods moving one way and coins
moving two - which is half a bidirectional trade with the other half
hidden behind role names.

`trade(a, b, moves)` is SIMPLER than what it replaced. Four things
became one function and four callers:

    buy     one side gives coin
    sell    the same with the columns swapped
    barter  both sides give goods, the difference settled in coin
    give    one side gives nothing

**A VALIDATING EXECUTOR, NOT A RULEBOOK.** Every decision is made in
Rust and arrives as a plan: which coins pay is `currency::pay`, what it
costs is `store::quote`, whether a stack merges or splits is
`objects::merge_into`. None of it is repeated in SQL, because a rule in
two places is a rule that will disagree with itself - proved four times
in this repo, most expensively in 028.

What SQL owns is the two things Rust cannot: that all of it lands or
none of it does, and that the plan is not lying. Moves name a SIDE
rather than an entity, so a plan cannot quietly deliver to a third
party who was never in the conversation.

**THE PEER CASE IS THE ONE WORTH NAMING.** A shop buys at 0.40 because
it resells for a living. Two players swapping a sword each are not, and
valuing both sides at 40% would mean an even trade left both of them
poorer - the goods would evaporate crossing the table. So
`store::worth` values a peer's goods at LIST in both directions, and an
even swap is even. That is what lets a party divide loot.

A MARKUP IS WHAT MAKES SOMEBODY A SHOP. No markup column, no merchant,
so two players get peer pricing with no flag anybody has to remember to
set.

**Verified in sections, each before the next was built:**

- The transaction: goods both ways in one call; whole rows moving with
  name and overrides intact; partial stacks splitting; arrivals merging
  into an interchangeable stack (5 arrows + 8 = 13 in ONE row); a
  failed move undoing its predecessors; third-party destinations
  refused; self-trade refused; named stacks refusing to split but
  moving whole.
- The pricing: 25 tests, including that a peer swap costs nothing and a
  merchant part-exchange settles the difference.
- `currency::allocate` MOVED OUT of the command file and tested. It was
  a rule wearing a wrapper - the step between deciding in KINDS and
  moving ROWS - and getting it wrong means paying with coins somebody
  does not have. Seven tests, including gold scattered across a purse,
  a pocket and a pack.
- End to end on live data, rolled back: Character1 traded a mace for a
  longsword at Halla's. 1500 in, 280 out (the mace at 0.40), 1220
  owed; paid 8 sp then 12 gp = 1280, with 6 sp change. Gold 18 to 6,
  silver 8 to 6, Halla 12 gold and 42 silver. Balances to the copper.
- The tab, through the stubbed rig: both sides populate, a purchase
  reads "you pay" in red, haggling sends its margin, switching to a
  peer hides haggling and says list price both ways, and Give sends
  `free: true`.

**SETTING UP A SHOP** is the Characters tab, on any row - PC or NPC: a
markup box and a disposition list under each name. 040 added those two
columns to serve `store::quote` and left no door, so every merchant
existed only inside a rolled-back probe - the same gap attunement had.
`set_merchant` is that door. A BLANK MARKUP IS THE OFF SWITCH, matching
040's nullable column: not a merchant is the absence of a price rather
than a price of zero.

Two parsers for disposition, on purpose. `parse` is for READING, where
an unexpected value is somebody else's problem and neutral is the safe
reading; `parse_strict` is for WRITING, where "freindly" silently
becoming neutral would leave a DM wondering why the discount never
applied.

Stocking one needs no new path: open sheet on the NPC makes them the
selected character, and the equipment panel's Add fills their shelves
and their till.

**CONSENT IS NOT MODELLED.** Both sides' rows move on one party's call,
because the DM runs the table and RLS already stops a player touching
what is not theirs. A player-to-player trade mediated by neither needs
an offer-and-accept handshake, and 041's header says so rather than
leaving it to be discovered.

**`trade` TRUSTS ITS CALLER ABOUT THE PRICE**, carried over from
`buy_object` and for the same reason - store.rs holds those rules and
duplicating them in SQL is the two-places problem. Safe while the DM
runs the table; not safe the day a player client calls it directly.

`buy` was DELETED rather than kept working. It called the dropped
`buy_object`, and it is a trade with one column filled - a second path
to the same place would be a second place to get it wrong.

## The armoury - BUILT (042, 043, 044-048, 050)

**57 WEAPONS, 193 TECHNIQUES.** 027 seeded the SRD tables, which are
deliberately short - 5e collapses a century of European polearms into
glaive, halberd and pike. 042 adds the twenty the SRD leaves out, all
low tech and all historical: the AD&D polearm family (bardiche, voulge,
guisarme, ranseur, bec de corbin, military fork, war scythe), four
swords that answer four different armours (falchion, khopesh, gladius,
estoc), and the rest.

MECHANICALLY THEY ARE 5e WEAPONS. Same classes, same properties, same
damage shapes - what makes them distinct is what they DO, which is
techniques rather than numbers. 2d4 appears here and not in the SRD on
purpose: it averages what 1d8 does and clusters harder, which is right
for a weapon whose point is reliable leverage rather than a lucky edge.

**043 gave every weapon at least three special attacks**, which was the
floor asked for. Before it, three weapons had techniques and fifty-four
had a damage die - so the item panel offered the Mace of the Deep Song
ten buttons and everything else one.

THREE TIERS, AND NOT THREE SIZES OF THE SAME HIT:

    level 1   the signature move - what the weapon is FOR
    level 3   a control effect - a save, a condition, a disarm
    level 5   the decisive one - bigger dice, better crits, or a cost

Built from what each weapon actually solved. A guisarme pulls riders
off horses; a ranseur traps blades in its side prongs; an estoc goes
through maille a falchion would only dent; a bec de corbin has a hammer
on one side and a spike on the other because plate respects one and
joints respect the other.

**MODES WERE VALIDATED, NOT ASSUMED.** A technique naming a mode the
engine never generates is not an error anywhere - it is simply never
offered, which is the worst kind of bug to find. The generator checked
every row against its weapon's own class before writing it, and the
live count of unreachable modes came back 0, dangling item keys 0, and
the fewest techniques on any armed weapon exactly 3.

**THE NET HAS THREE NOW, and 043 was right to refuse it.** 043 left it
empty because it deals no damage and its whole function is `spc`,
meaning restrain, with no condition system to restrain anybody with -
and three attacks rolling damage for a weapon with no damage would have
been worse than the honest absence. 048 did not overrule that; it
changed the weapon. A retiarius net was not a bedsheet, and giving it
the lead weights real ones carried makes it a 1d4 thrown weapon whose
damage is the rim rather than the mesh. The gap closed by fixing the
premise, not by filling it in.

The blowgun is still out, on the same reasoning, and nothing has
changed its premise.

**048 READ EVERY WEAPON BACK AGAINST ITS OWN LADDER**, which is what
found four filed by weight rather than by what they are. 044 through
047 are the same exercise on sizes and slots: the trident filed small
beside a spear, a quiver that held two hundred arrows because an arrow
was 0.05 of a slot, and platinum too big to go in a coin purse - which
was 039 inheriting 036's `med` default and nobody noticing that a coin
is tiny.

**050 PUTS MOVES ON ONE OBJECT.** A technique can name an `object_id`
instead of an item key, so a particular sword carries moves no other
sword has. Two live so far. Removal is a tombstone rather than a
delete, because these rows share a table with 006's catalogue and a row
that is simply gone lets the catalogue's version reappear.

**SPECIAL TEXT IS PROSE THE ENGINE DOES NOT READ.** Bleed, stun, armour
reduction and forced movement are written for the table to adjudicate,
exactly as 006's rows are. The dice, the crit range and the fumble
range ARE mechanical and do fire. Nothing here pretends a condition
system exists.

## Editing one object - BUILT (049)

The viewer showed nine facts and the editor reached three. You could
rename a greatsword, change how many there were and call it unusually
large, while the panel beside it reported weight, damage and properties
nothing could touch.

**AN EDITED OBJECT IS A UNIQUE OBJECT, NOT A NEW CATALOGUE ROW.** The
obvious move is a campaign-scoped `items` row per edited thing - 008
did exactly that and said "a +1 sword is not a sword". 026 called it
the wrong shape and it still is: the catalogue stops being a list of
what EXISTS and becomes a list of what has ever happened.

**SEVEN COLUMNS, NOT A JSONB BAG.** A bag takes any fact with no
migration and was the first instinct. Two things argued it down. It is
untyped - `damage_denomination` is checked >= 2 on `items` and a bag
would carry 0 happily until something divided by it, so every
constraint would need rewriting in Rust, which is 028. And 036 ALREADY
CHOSE with `size_override` and `holds_size_override`; a bag beside them
is two mechanisms for one idea, and folding them in means rewriting
twenty-two call sites across six files written the day before.

If the list ever passes ten, the bag wins, and 049's header carries the
argument for switching.

**LISTS REPLACE RATHER THAN MERGE**, which is the only reading that can
REMOVE something. A greatsword reforged light has lost `hvy`, and a
list that only adds could never say so - so an override carries the
whole set or none of it. Blank clears the override; the word `none`
sets it to empty. Those are different facts and both are needed.

**ONE MERGE FUNCTION, TWO SCREENS.** `Overrides::apply` runs in
`load_loadout` before proficiency and modes are derived, so a greatsword
that lost `hvy` really stops being heavy rather than merely displaying
as light. `holders::resolve` calls the SAME function - the manager used
to read damage and properties off the catalogue, which meant an edited
object displayed its type's numbers while the sheet rolled its own. The
two-places problem, on screen.

Verified live and rolled back: a greatsword at 2d6 slashing [hvy, two]
became 2d8 slashing/fire [two] at 4.5 lb worth 400 - with `hvy` gone,
which is the case a merge could not express. The editor was driven
through the rig: seven fields carrying their values with the type as
placeholder, and a save sending blank to clear, `none` to empty and a
value to set.

## Pick up here

**Be clear about what is and is not done.** The foundation is square
and the combat loop runs: the access model, the dice, the sheet
resolver, the prose, equipment, attacks, hit points, dying, the DM's
screen, and monsters that are individuals rather than views of a type.

Inventory is now real on both sides: a catalogue to pick from, a
designed way to acquire, and objects that can be named, split, dropped
and destroyed.

What AppSheet did that this still does not is *deliver*. `rolls.status`
goes `pending -> resolved -> delivered` and nothing in this codebase
moves a row to `delivered`. There is no die art, no rests and no spell
slots, and the UI is a test rig. A PIN now saves you
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

1. DEATH SAVES ON THE TURN, which is what initiative was for.
   051 built the order and the round; 015's saves still happen on a
   button, and that is now the only place the build departs from the
   rule as written. The turn exists for them to happen on.

   The other half left over from the encounter work is ATTACKING A
   THING. 052 made an object targetable as a DC and deliberately
   stopped there; a door with an AC and hit points wants vitals on
   objects and hp_events against a subject that is not a creature. Read
   052's header first - it states what it refused to decide.
2. EDITING a creature, now that viewing one works. Renaming is done;
   the rest is scores, hit points, AC and kit, all of which are plain
   columns on a character the DM already owns. Then "save as template",
   which is the same copy `instantiate_npc` does, pointed the other way
   - and it is worth doing, because building an interesting goblin in
   play and keeping it is how a DM actually works.
3. ACCESS CONTROL, which is the next subsystem, the one with
   something already waiting for it, and now the one with a named first
   piece. `acquire.rs` has held the Take rule - tested - since the 22nd
   and nothing calls it. Locations are what give it somewhere to apply:
   a locked room, an owner, a witness, a crime. It is also what NARROWS
   033's deliberately generous rule that anything loose is anyone's to
   take, stated in the migration as a starting point rather than a
   decision that theft is free.

   **START WITH LOCKED AND OPEN ON CONTAINERS.** `containers.rs` has
   listed them as missing since 032 and they are the gate that makes "a
   container in a location a player can access" mean more than "a
   container in a location" - right now an unlocked chest and a locked
   one behave identically. Reach was the half that could be built
   without them and is done; this is the other half. It is a small
   migration and one more question in `guard_capacity`.
4. The roll-to-challenge payoff. `actions.target_challenge_id` is
   written and nothing reads it, so the iron lock still cannot say
   whether it has been picked. The derivation is a query away: a
   successful action pointing at the challenge, later than its
   `reset_at`. See 011's comment on that column.
5. Die art on the roll - read the equipped set from character_dice,
   look up dice_faces for the natural d20, snapshot image_url and
   set_key onto the roll at insert, per the record principle.
   `narrative.rs` is the shape to copy, down to caching it on the sheet.
6. The rules modules still unported: rests, spell slots - death saves
   landed with 015 -
   each isolated in its own Apps Script file, each wants its own Rust
   module with tests. Each also wants a `resolve_request` key, and the
   vocabulary already has room for `death` and `spell`.
7. Delivery - whatever moves a resolved roll to `delivered` and puts it
   in front of the table. This is the piece that closes the loop
   AppSheet closed, and nothing else on this list matters as much.
   **It has now been deferred twice.** Note that and decide
   deliberately rather than by drift.
8. Session persistence, PROPERLY. A PIN now stands in front of it, so
   a restart costs four digits instead of a password - but the session
   still lives in memory and the refresh token sits in a plain file in
   the app data directory. The real answer is unchanged and is the OS
   keychain: Credential Manager, Keychain, the Android Keystore. Then
   the token is held by the operating system, the PIN becomes a second
   factor rather than the only one, and pin.rs changes from "where the
   token lives" to "what unlocks it". Read pin.rs before trusting the
   PIN with anything.
9. ENCUMBRANCE, which is what `weight` is actually for. Every one of
   the 68 catalogue rows has carried a weight in pounds since 027 and
   032 and nothing read it until the manager screens printed it. 036
   deliberately did NOT give containers a weight capacity, because
   `slots` is already the "how much fits" measure and is already
   fractional for exactly that purpose - a coin is 0.2 of a slot, so a
   5-slot purse holds 25. Two numbers meaning almost the same thing is
   two numbers to keep in agreement. Weight's own job is what a PERSON
   can carry, which is STR x 15 in 5e and is a rule about a person
   rather than about a sack.
10. A real phone-first UI. What exists is a desktop test rig.
11. Android via `npm run tauri android init`. iOS needs a Mac.

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
- **AN ENCOUNTER CANNOT BE DELETED, AND THAT IS THE DESIGN.** Rolls
  point at encounters and actions point at rolls, so a delete would
  either cascade through a session's history or be refused - and the
  refusal is the honest answer, so 053 never offers it. "Retire from
  the list" sets `archived` and the fight drops off the list with a
  way back. Archived is NOT a status: 034's four words say what a
  fight IS, and being off a list is housekeeping. Anything that reads
  encounters and wants only the live ones has to filter `archived`
  itself - `list_encounters` returns both.
- **`encounters.narrative` is prose the engine will never read**, the
  same contract as 043's special text and 006's lines. If a rule ever
  needs to fire off what a fight is about, it needs its own column;
  parsing that field would be the AppSheet mistake again.
- **Nothing gates on the turn**, deliberately - see Combat. If a later
  decision reverses that, `initiative.rs::next_turn` and the strip are
  where the order lives, and every write path would need the gate, not
  just the obvious one. See "A RULE ENFORCED IN ONE COMMAND IS NOT
  ENFORCED".
- **Two swings in one round are SHOWN, never refused.** If that is
  ever reversed, the count is `spent.rs` and the gate belongs beside
  it, in Rust, where it can be tested - not in a check constraint and
  not on the screen. Every write path would need it, not just the
  obvious one.
- **`take_object` is the last registered command with no caller**, and
  an uncalled command is a defect rather than a spare. Five others were
  settled on that rule: `set_encounter_location` and `destroy_object`
  got the screens they were waiting for, and `loose_objects`,
  `load_techniques` and `list_roster` were deleted. This one stays
  because the Take rule in `acquire.rs` is the access-control work it
  belongs to - wire it or retire it when that lands.
- **Versatile weapons store their second die and nothing reads it.**
  027 added `items.versatile_number` / `versatile_denomination` so a
  longsword row is not a lie, but the engine still offers the 1d8. The
  work is a fourth `Mode`, which means widening `techniques_mode_check`
  and teaching `resolve_request` which die to take. Own migration, own
  tests.
- **`rch` and `spc` are data with no rule.** Reach is a grid fact and
  there is no grid; special means "read the entry". They are in the
  catalogue so a glaive can be told from a greatsword before either
  rule exists.
- **The blowgun is still missing** and will be until something can
  express flat damage. Several other things will want that column.
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
