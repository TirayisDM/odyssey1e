# odyssey1e - session handoff

**Written 2026-09-17. Updated 2026-10-06 after the creature stack:
what a thing is, a monster that says what it resists, a statblock you
can instantiate as a character, a creature as a file, and a bestiary of
134. 2026-10-03 brought the casting stack before it: the spell
catalogue, cleric prayers, effects that reach the dice, damage
resistance, and saves you can roll from the ability row.**
Read `README.md` first for how to run it; this
file is only where things stand and what comes next.

This is a chronological log and it is long. A newcomer should read
`README.md`, then **"Pick up here"** at the bottom of this file, and
treat everything between as history to search rather than to read.

---

## Machines

| | |
|---|---|
| Desktop (execution) | `C:\Users\tiray\Dev\odyssey1e` |
| Laptop (planning) | `C:\Users\tiray\odyssey1e` |
| Remote | `github.com/TirayisDM/odyssey1e` (**public**) |

Both on Rust 1.98.1. Commit on either, `git pull` on the other. Same
two-machine pattern as `odyssey-engine`.

**The repo is public.** It said "private" here until 2026-10-03 and had
been public since it was created. That is survivable - the only
credential in the tree is the Supabase publishable key, which is built
to ship in clients, and the security boundary is RLS. It is not
survivable if a service-role key or the database password ever lands in
a commit. Nothing else needs to be kept out.

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

**538 tests, zero warnings.** `cd src-tauri && cargo test`.

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
055 a character has a calling - the twelve classes, and the hit die a
    player character's hit points actually come from
056 a character has a people - species, their modifiers, and an honest
    line between what the engine applies and what a DM still must
057 how tall is a people - height in feet, playability, one size ladder
    instead of four, and the Unt'gar
058 what a character looks like - the body, spoken and written kept
    apart, and a trait that can cost you something
059 a short stride is a cost - the Unt'gar's 25 feet, said where a
    player will look for it
060 an action knows whose turn it was - stamped, so out-of-turn is a
    fact rather than a comparison against a pointer that has moved
061 how many swings do you get - Extra Attack, and what an action cost
062 the other three slots - a bonus action, a reaction and a free
    interaction, spent by a tick rather than by a roll
063 a held action names its place - holding is a position in the order,
    not a trigger
064 a goblin has a calling too - NPCs get a class, and the twelve in
    place were swept
065 an instance inherits its calling - instantiate_npc carries it, so
    the thirteenth goblin is not born classless
071 what a thing looks like - items.description filled on all 89
    catalogue rows. The column existed from 004 and nothing ever wrote
    to it; 070 put it on screen and turned a quiet absence into a
    visible one. Guarded on NULL and on game_id IS NULL, so it is
    re-runnable and takes nothing back from a game that authored its own
073 a character is more than one thing - character_classes, one row per
    class, plus sync_character_level keeping characters.level and
    class_key in step from it
074 something to play it on - nine instruments, and `instrument` added
    to the items.kind check constraint that refused the first insert
075 trained with a tool - characters.tool_profs, and classes.tool_choices
    saying a bard is owed three
076 karma and the audience - classes.karma_skills (bard is {ins,prf}),
    and the seven-row audiences catalogue that is the chart's other axis
077 the trigger surface goes back to zero - the EXECUTE revoke that 002
    and 017 established, applied to the fifteen trigger functions
    written since, and three search paths repinned to ''
078 a class grants its saves - the two saving throws a class gives,
    which 055 named and nothing read
084 where a thing is worn - `objects.slot` replaces the `equipped`
    boolean, because a hand and a back are different answers
085 what hangs off a belt - the hip slot, and what is small enough
086 put the backfill somewhere possible - 084's placement rule,
    corrected: one weapon in hand, the rest at the belt
087 what a class gives you - class_features and character_choices.
    Features are DERIVED from the catalogue and the level; a choice is
    the only part stored
088 the twelve classes level by level - 176 feature rows
089 two functions that still wrote `equipped` - unheld_is_unequipped
    and instantiate_npc, which 084 broke and nothing compiled against
092 a game knows what time it is - games.tick in six-second rounds,
    last_long_rest, hit_dice_spent, character_uses
093 what runs out and what brings it back - uses and recharge for the
    features that have them
094 something that is true for a while - the effects table, expiry as
    a tick comparison, and the four stacking rules
095 one instantiate_npc, not two - dropping the overload 089 created
    by reordering its parameters
096 the Fjell'gar come down the mountain - the third people seeded,
    DEX +2 WIS +1, AC 12 + DEX unarmoured, Athletics granted, and five
    traits written down that nothing can apply yet
097 the Ny'ook live in the moment - the fourth, DEX +2 CHA +2, Small,
    Acrobatics granted, and the first LOWERED ability ceiling: STR 13
098 a people can be hard to enchant - species.spell_save_bonus, and
    `wis save vs spell` as a request the engine understands

(066-070, 072, 079-083, 090, 091 and 099 are code-only changesets
with no migration - the numbering is continuous across both, which is
why there are gaps here. 099 is the species ability ceiling, enforced
in Rust at both write paths and needing no schema at all. A gap is expected; a NAME in the database with no
file is not, and there have been five - see the drift trap.)

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

**THE DATABASE CAN HOLD SCHEMA THAT GIT DOES NOT. CHECK AFTER EVERY
PULL.** Five migrations have now been applied to the live database
with no file in the repo, or with a file holding no SQL:

    087  class_features and character_choices   file missing
    089  the two functions 084 broke            file had COMMENTS ONLY
    092  the game clock                         file missing
    093  what runs out and what brings it back  file missing
    094  effects                                file missing

Every one was recovered on 2026-10-02 from
`supabase_migrations.schema_migrations`, which keeps the statements it
ran - in this project's case with the comment headers intact, so the
recovered files are the originals rather than a reconstruction. All
five were verified by MD5 against that record before being committed.

WHY IT MATTERS EVEN THOUGH THE APP WAS FINE. The running app reads the
live schema and never noticed. What broke was the chain: 088 seeds the
table 087 creates, so the migrations as committed could not run in
order, and a fresh clone could not build this database. That is also
exactly what a second developer gets handed.

THE CHECK, after every pull and after any session that applied a
migration:

    select version, name from supabase_migrations.schema_migrations
     where version > '<the last one you know about>' order by version;

against `ls supabase/migrations`. A name in the database with no file
is the fault; gaps in the NUMBERING are not - several numbers are
code-only changesets, which is recorded under the migration list.

AND A FILE THAT IS ALL COMMENTS COUNTS AS MISSING. 089's header said
the applied SQL was in the migration history and the file recorded
what it did - which reads as deliberate and leaves a clone running
084's column drop with neither repair after it. `grep -c -v '^--\|^$'`
over the migrations directory finds that one.

**A `create or replace function` THAT REORDERS THE PARAMETERS CREATES A
SECOND FUNCTION.** Postgres identifies a function by its argument types
IN ORDER, so this is not a replacement:

    022  instantiate_npc(p_npc_key text, p_game_id uuid, p_label text)
    089  instantiate_npc(p_game_id uuid, p_npc_key text, p_label text)

It is an overload beside the old one, and `create or replace` says
nothing, because nothing about it is wrong. The old body kept the
column 084 had dropped.

WHAT MAKES IT BITE IS POSTGREST, which dispatches an RPC on parameter
NAMES. Both functions answered to the same three, so every call became

    function public.instantiate_npc(p_npc_key => text, p_game_id =>
      uuid, p_label => unknown) is not unique

and every enrolment failed - including the one 089 was written to fix.
A repair that leaves the original fault in place and adds an ambiguity
on top of it. 095 drops the 022 signature by exact argument types,
which is the only way to name one of two functions sharing a name.

The lesson is narrow and worth keeping: changing a function's
SIGNATURE is a drop and a create, never a replace. Count them
afterwards:

    select proname, pg_get_function_identity_arguments(oid)
      from pg_proc p join pg_namespace n on n.oid = p.pronamespace
     where n.nspname = 'public' and proname = '<the one you changed>';

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

**FIVE SECURITY DEFINER advisor warnings are expected, and they are
all `authenticated`, never `anon`:**

    is_game_member      called BY the policies in 001 - revoke it and
    is_game_dm          every one of them fails closed
    holder_character    the same, for the object policies
    holder_is_a_location
    join_game           the one a player calls on purpose

Do not "fix" those. Run the advisor after every DDL change anyway.

**THIS ENTRY SAID THREE UNTIL 077, AND THE ADVISOR SAID FIFTEEN.** 002
and 017 revoked the default EXECUTE on every function they could see,
and the pattern then lived nowhere but in those two files - so the next
fifteen trigger functions were written without it, one or two at a
time, each one looking fine on its own.

The risk was small: Postgres refuses to run a trigger function called
any other way, so the grant was reachable and not useful. The cost was
the note. A reader opening a list of fifteen against a line promising
three learns that the list is noise, and stops reading it - which is
exactly when the sixteenth will not be a trigger.

077 revoked all fifteen and the count is now zero. **If this entry and
the advisor ever disagree again, the entry is the thing that is
wrong.** Count with:

    select p.proname from pg_proc p join pg_namespace n
      on n.oid = p.pronamespace
     where n.nspname = 'public' and p.prosecdef
       and p.prorettype = 'trigger'::regtype
       and (has_function_privilege('anon', p.oid, 'EXECUTE')
         or has_function_privilege('authenticated', p.oid, 'EXECUTE'));

**EVERY SECURITY DEFINER FUNCTION PINS `search_path = ''`** and writes
each table name out in full. 061, 063 and 073 pinned `public` instead;
077 put them back. With an empty path an unqualified name fails to
resolve at creation, which turns a typo into an error. With `public`
it resolves - and the temporary schema is searched ahead of it for
tables, so a session holding a temp table named `characters` would have
had `sync_character_level` update that one and report success.

---

## House rules, marked so nobody takes them for 5e

**REACH IS DERIVED FROM SIZE** (057, size.rs). 5e does NOT do this - a
Large creature has 5 feet of reach unless its own statblock says
otherwise, and reach is a property of the creature and its weapon rather
than of its size band. In a campaign running from a 2-foot rodent people
to a 25-foot Imiear that is untenable, so `SizeClass::reach_ft` scales:
5 / 5 / 5 / 10 / 15 / 20 across the six rungs. Anything that wants RAW
behaviour has to override it per creature, and nothing does yet.

**TINY CREATURES AND HEAVY WEAPONS** (057). 5e gives Small creatures
disadvantage with Heavy weapons and says nothing about Tiny, because it
has no Tiny player characters. `size::heavy_weapon_is_awkward` extends
the rule down rather than leaving a two-foot creature swinging a
greataxe unremarked. Not wired to the attack path yet - the wielder's
size does not reach it.

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

## Classes - BUILT (055, class.rs, commands/characters.rs)

**A character made through the app had no hit points.** Not zero -
NULL, an empty space on the sheet where a life goes. `create_character`
took a name and inserted a name; level defaulted to 1 and everything
else to nothing. Snot and Unnamed stood that way for weeks and nothing
complained, because nothing was asking.

The chain was: no size, therefore no hit die, therefore no maximum.
029 made the die come from SIZE, which is the Monster Manual's rule and
right for monsters. `vitality.rs` has carried the other half as a
comment since:

> WHAT THIS IS NOT. A player character's hit points are not this. 5e
> maxes a PC's first hit die and rolls the rest, and the die comes from
> class rather than size.

055 is the table that comment was waiting for.

**TWO RULES, NOT ONE WITH AN EXCEPTION.** `vitality::average_hp` is the
monster rule - level times the average of a size's die, floored.
`vitality::pc_hp` is the character one, and it differs in both halves:
the die comes from class, and the first level is MAXIMISED rather than
averaged. They also round opposite ways - a monster's total floors, a
character's per-level value is 5e's printed fixed number, which is the
average rounded up. A test asserts they disagree, so nobody collapses
them later: same d8, same level 5, no Constitution - a monster has 22
and a character has 28.

Levels after the first take `die / 2 + 1` rather than a roll, because a
maximum has to be RECOMPUTABLE. A rolled maximum is a historical event;
move the level and there is nothing to derive it from. That is 029's
argument for monsters, applied to characters.

**THE VOCABULARY WAS THE RISK, and it was checked rather than assumed.**
`equipment::is_proficient` matches weapons on `sim`/`mar` or an exact
item key, and armour on `lgt`/`med`/`hvy`/`shl`. A class row saying
"simple" or "light" would make a Fighter proficient with NOTHING and
report nothing anywhere - this codebase's most expensive defect class.
All 12 seeded classes were verified against the live catalogue: every
weapon token is a class prefix or a real item key, every armour token
a real category, every skill option a real `skills.key`, every save a
real ability code. Zero mismatches.

**A character is now born finished.** Pick a class and the row arrives
with a size, the class's weapon and armour proficiencies snapshotted
onto it, and an `hp_max` derived from the die and the Constitution the
seed trigger wrote. Picking no class is still allowed - a blank sheet
is a real thing to want - and then `hp_max` stays NULL, which honestly
says nobody has decided what this character is.

Proficiencies are COPIED rather than looked up, on 001's principle: what
a character is proficient with is a fact about the character, and a DM
who rewrites the class catalogue next month has not retrained anybody.

Size defaults to `med` rather than NULL. Every playable SRD species is
Small or Medium, the difference changes no rule the sheet reads today,
and the alternative is the absence that broke Snot.

**WHAT 055 DELIBERATELY IS NOT.** No subclasses - they arrive at level 3
and each is a bundle of features wanting its own table. No class
features: Second Wind, Sneak Attack and Rage are rules with resources
and timing, and a `features text[]` would be a list of words no code
could act on. No spellcasting, with no spell table to point at. No
multiclassing - `class_key` is one value, and 5e multiclassing needs
levels per class, which is a join table. No ASIs and no starting
equipment.

`characters.class_key` references `classes.key` BY VALUE with no foreign
key, for the reason 004 recorded: the two partial unique indexes that
make nullable tenancy work cannot back one.

**THE LEVEL BUTTON NOW RIPPLES FOR CHARACTERS TOO.** 029 asked for it -
"this should ripple through their HPs" - and got half: `set_actor_level`
recomputed a monster's maximum and `set_level` wrote `{"level": n}` and
stopped. A player character's maximum was therefore whatever it had been
at CREATION, which is level 1 with the Constitution 10 the seed trigger
writes before anybody chooses anything. Garn, a level 5 Barbarian with
CON 13, sat on the sheet with 12 instead of 45.

`commands::characters::rederive_hp_max` is the one place that answer is
worked out, called from `set_level` AND from `set_ability` when the
score is `con` - because Constitution applies per level, so moving it
by one moves a level 5 character by five. It returns None and writes
nothing when there is no class to derive from: a stated maximum is a
real thing, and inventing a d8 is not.

`commands/characters.rs` also took `list_characters` and
`create_character` out of lib.rs, which is commands/mod.rs's own rule -
a group migrates when it is being worked on anyway.

## Species - BUILT (056, species.rs, the Species tab)

**THIS CAMPAIGN IS ALL CUSTOM SPECIES.** Not 5e's list plus some - the
whole roster is Dave's, and the Unt'garoth are the first. That is why
`species` has no SRD layer underneath it: the global rows ARE the
campaign's. The nullable tenancy is there anyway, because a second
campaign wanting its own Unt'garoth is what it is for.

055 gave a character a class and therefore a hit die. 056 gives them a
people, and with it the rest of what a sheet is made of.

**BASE AND EFFECTIVE ARE DIFFERENT FACTS.** A species bonus is NEVER
written into `character_abilities.score`. The stored number stays what
somebody rolled; `species::effective_score` adds the bonus on the way to
the sheet, and `Ability` carries `base`, `bonus` and the effective
`score` separately. Folding +2 into the row would destroy the base, so
changing species later would double-count, and an 18 would be
indistinguishable from a 16 with a people behind it. 049's tri-state
argument in another costume.

The editor writes `base`; everything else reads `score`. That is the
whole reason the split exists, and the ability box on the sheet shows a
small `+2` beside the modifier so an 18 reading +5 does not look like
broken arithmetic.

**ONE PLACE APPLIES IT.** `character::apply_species` raises the
abilities map once, immediately after the read. Every modifier in the
app comes from `Ability::score` through `ability_mod_of`, so skills,
saves, attack rolls, carrying and armour class all pick the bonus up
without any of them knowing species exist. The alternative - each site
adding it - is the two-places problem `supabase::numeric` was about.

**WHAT IS APPLIED, AND WHAT IS ONLY WRITTEN DOWN.** This is the honest
column, and `traits.applied` carries it to the screen:

    APPLIED        ability bonuses, ability maxima (STR 21, not 20),
                   size, Powerful Build (one step up carry.rs's size
                   ladder), granted skills, unarmoured AC
    NOT APPLIED    speed 40 - THERE IS NO MOVEMENT SYSTEM
                   fire/cold resistance - THERE IS NO RESISTANCE SYSTEM
                   the advantage traits - Adv/Dis is a human choice on
                   the roll screen with nothing to fire it
                   cannot-swim, high-altitude, disease resistance

A trait a DM adjudicates is fine. A screen that hides which ones those
are is not, so the viewer tags every trait `applied` or `DM applies`
and colours the two differently. 054 refused to guess at 193 action
costs for the same reason.

**POWERFUL BUILD IS A SEPARATE SIZE.** `Sheet::carry_size` is the size a
character CARRIES as; `vitals.size` is the size they ARE. Only carrying
moves - not reach, not cover, not the hit die, not what a container
admits. `species::bump_size` walks `vitality::size_rank` rather than
inventing a second ladder.

**UNYIELDING DEFENSE** is `equipment::armor_class`'s new `unarmored`
argument: a base and an already-resolved modifier, applied ONLY when no
body armour is worn. A floor, not a bonus - putting on a breastplate
goes back to the armour's number, and a shield still counts on top.
Tests assert all three.

Verified live and rolled back, creating an Unt'garoth Barbarian: size
lg, carries as huge, STR 10 to 12, CON 10 to 11, hit points 12, carry
capacity 720 lb, Athletics granted, unarmoured AC 12. The Species tab
was driven through the rig - facts render, and the three traits come
back tagged applied / applied / DM applies.

**ONE CONTRADICTION IN THE SOURCE, FLAGGED NOT BURIED.** The species
document says both "their strength to extend to 20 naturally" and, under
its own heading, "Maximum of 21". 21 is seeded, because the later
passage is the specific one with a mechanic and a worked explanation
against a clause in a summary sentence. It is a design question, and
056's header says so where it can be found.

Not here: a species EDITOR. Species are authored in the database for
now; the tab reads. Authoring wants a form with validation against the
same vocabularies, and the reading half is what was blocking play.

## Size - ONE LADDER (057, size.rs)

**THE CAMPAIGN RUNS FROM TWO FEET TO TWENTY-FIVE.** A rodent people at
2', the Unt'gar at 4.5', the Fjell'gar at 5', the Unt'garoth near 8',
the Jotun at 18', the Imiear at nearly 25'. Size is not a footnote in a
world shaped like that.

**THE LADDER WAS WRITTEN OUT FOUR TIMES.** `carry::size_multiplier` held
the capacities, `vitality` held the dice and the ordering,
`species::bump_size` held its own array of the six words, and main.js
briefly held a fifth. vitality.rs had warned about exactly this in a
comment - "two copies of one vocabulary waiting to disagree about
whether grg exists" - and then a third appeared anyway, because there
was nowhere for the second to move to. size.rs is that place. Same
lesson as `supabase::numeric`, which cost three bugs.

One table now carries, per rung: rank, space, reach, carry multiplier,
hit die, and the height band. Everything else asks.

**HEIGHT IS THE FACT; THE CATEGORY IS A CONSEQUENCE.** A species states
its feet AND its rung, and `derived` is computed from the height so the
two can be compared. A document saying Large with a height reading
Medium is a contradiction the viewer SAYS rather than one of them
silently winning.

Derived off the MIDPOINT of the band, not the minimum, and the
Unt'garoth are why: 7 to 10 feet, and 5e's Large starts at 8, so their
shortest adult is Medium and their tallest is Large. A people is typed
by its typical adult. The midpoint is 8.5, which is Large, which is what
their document says. Off the minimum it would have reported a
contradiction for every species whose range crosses a line - which is
most of them.

**THE ROUNDING THAT BITES, AND IT IS A DECISION NOT A DEFECT.** The
Jotun at 18 feet and the Imiear at nearly 25 are BOTH Huge, because 5e's
Huge band runs 16 to 32. Seven feet apart, and not one number between
them differs - same reach, same space, same carrying. There is a test in
size.rs asserting it, so it cannot be forgotten.

If that is wrong for this campaign the change is to scale the CONTINUOUS
facts - reach, space, carrying - off `height_ft`, and keep the category
only for the discrete rules that need rungs: grappling, squeezing,
mounts. `height_min_ft` and `height_max_ft` are what such a change would
read, which is half the reason they are stored.

**RULES THE LADDER NOW CARRIES**, tested, three of them not yet wired
because the systems they need do not exist:

    can_grapple           a target may be at most ONE size larger. The
                          rule that stops a 2-foot rodent wrestling a
                          Jotun. No grapple action yet.
    squeeze_into          one rung down. No map yet.
    can_carry_rider       a mount must be at least one size LARGER than
                          its rider. This campaign has a rodent people
                          who ride ravens, so it will want this.
    heavy_weapon_is_awkward   Small and Tiny. The attack path does not
                          know a wielder's size.

**NOT EVERY PEOPLE IS A PLAYER CHARACTER.** `species.playable` keeps the
Imiear out of the creation picker while leaving them a full species
everywhere else. A fact about the species, not a permission check, so it
lives on the row rather than in a policy.

**THE UNT'GAR ARE SEEDED WHOLE** - CON +2, INT +1, Medium, 25 feet,
History granted. Stonecunning gets the Enduring Might treatment: the
proficiency is granted, the DOUBLING for stonework is a DM call, because
nothing here can tell a question about stonework from one about kings.
Darkvision, Mineral Sense, Trade Savvy and Environmental Resilience are
written down and applied by nothing - no vision system, no rest system,
and Adv/Dis is a human choice.

**A STATED CEILING IS ENFORCED ON ALL FOUR PATHS.** 056 built
`ability_maxima` for the Unt'garoth's RAISED Strength ceiling of 21;
097 lowered one for the first time - the Ny'ook cannot naturally
exceed 13 - and 099 closed the two doors still open:

    a species bonus   `effective_score` will not add past it
    an ASI            `apply_bumps` clamps to the ceiling
    creation          `create_character` refuses an assignment over
                      it, before anything is written
    an edit           `set_ability` refuses the same, because a sheet
                      editable to 16 after being created at 13 is not
                      capped at all

ARITHMETIC CLAMPS, A WRITE REFUSES, and the split is deliberate.
`effective_score` still lets a base above the ceiling through, because
it answers "what does the species ADD" and a DM who types 24 means 24.
A cap on what somebody may HAVE is a different question and it has to
say no out loud - quietly lowering 16 to 13 would leave a player never
learning why their rolls did not land.

ONLY A STATED MAXIMUM REFUSES, AND THE TARRASQUE IS WHY. A statblock
is a character and monsters run to Strength 30. No species, or a
species with no opinion about that ability, means no refusal;
imposing 5e's default 20 on everything would make the dragon
unwritable in order to enforce a rule about the Ny'ook.

NATURAL IS THE LOAD-BEARING WORD. Dave's rule is that a spell or a
magical item MAY carry somebody over their cap. Nothing does yet -
094's effects do not reach ability scores - and when something does it
must add on top at READ time rather than route through `apply_bumps`,
which clamps because training is exactly what a ceiling is about.

**NOT SEEDED: the Jotun, the Imiear, and the rodent people.** Their
heights are known and nothing else is. A row with a name and six
defaults is worse than no row, because it is pickable and gives a
character nothing. They land when their documents do; 057 is the
framework that will take them - and 096 is the proof it works, seeding
the Fjell'gar the day their two documents arrived.

**THEY ARE THE FJELL'GAR, not the "Felligar".** This file called them
that from 057 until 096, which is how the name was heard rather than
how Dave writes it. The key is `fjellgar`, dropping the apostrophe the
way `untgar` and `untgaroth` do.

## The Description subtab - BUILT (058)

**THE SHEET HAS SUBTABS NOW**, and this is the first of about six.
`#sheet-tabs` runs on `showSub`, the same helper the Characters tabs
have used since 033 - so the next one is a button and a pane and
nothing else. Stats keeps what the sheet already had; Description is
new. Combat, spells and background are the obvious next three and
nothing about them needs deciding yet.

**THE SPECIES STATES A RANGE, THE CHARACTER STATES A VALUE.** That is
the shape of the whole migration. The Unt'garoth run 7 to 10 feet and
400 to 900 pounds; Garn is one specific height. `characters.height_ft`
and `weight_lb` hold his, `hair`/`skin`/`eyes`/`description` hold the
rest, and the panel shows the value AGAINST the band - "9 ft ·
Unt'garoth run 7-10 ft" - so a short one reads as short rather than as
a number. Same separation as 049's overrides and 056's bonuses: the
general fact and the particular one stay apart so both can be true.

Nothing is defaulted. An unstated height is unstated; filling in the
middle of the species band would put a fact on the sheet that nobody
decided. Height and weight are NOT validated against the species
either - an Unt'garoth of six feet is short for their people, not
illegal, and refusing them would be the app overruling a DM about their
own world. The panel says so; it does not object.

**SPOKEN AND WRITTEN ARE DIFFERENT FACTS.** `languages text[]` could
only carry a name. 5e writes "speak, read and write X" as one phrase and
most tongues are all three, but the interesting ones are not: a language
with no script, a dead one read and never pronounced, a character who
speaks four and reads none. A tongue is now `{name, spoken, written}`
and the two booleans are separate because they are separately true.

A character carries their OWN on top of their people's, and the panel
shows the union with where each came from. A MISSING BOOLEAN READS AS
TRUE here, which is the opposite call to `Trait::applied` and right for
the opposite reason: naming a language is claiming it, and defaulting to
false would silently mute somebody over an unfilled field.

**A TRAIT CAN COST YOU SOMETHING.** 056's traits were all upside,
because the two species seeded first mostly are. An Unt'garoth CANNOT
SWIM - their own document says the density does not permit it - and
showing that in the same list, in the same colour, as "resistance to
fire and cold" is a screen lying by arrangement. `kind` is 'feature' or
'drawback', the panel splits them, and a drawback's rule is red rather
than green.

DELIBERATELY NOT MECHANICAL. Nothing computes differently from `kind`;
it is presentational, and saying so is more honest than implying a
system that does not exist. It is a STRING rather than a bool so a third
kind can arrive without a migration rewriting every row.

ONE drawback is marked - the Unt'garoth's Dense Mass - because it is the
only one either document states. The Unt'gar have none written down, and
inventing some to balance the two would be designing Dave's game for
him. An empty Drawbacks section is an honest answer.

Verified through the rig against a stubbed Unt'garoth: the value-against-
band line renders, features and drawbacks land in their own sections with
the drawback carrying its own border, and a tongue reading "Old Jotun ·
cannot speak · reads / writes · learned" proves the split does what it
was built for. The save sends every field, with an emptied one clearing
and the untouched ones carrying their current values - 036's convention.
Verified live and rolled back on Garn: every column takes the write, and
`to_json(height_ft)` is `9.0`, a bare number, read through
`supabase::numeric_at` like every other numeric in the app.

## The sheet's subtabs - FOUR OF SIX, IN CHARACTERS (058, 059, 067)

**067 MOVED THE WHOLE SHEET OUT OF PLAY** and into the Characters tab,
where the list you pick a character from already is. Play keeps Rolls
and nothing else.

A SIBLING OF `#chars-panel`, NOT A CHILD. That panel is hidden for
anybody who is not the DM, so nesting the sheet inside it would have
taken every player's own sheet away - the Characters tab would have
become a DM tool and a player would have had nowhere to read their own
numbers. The sheet sits beside it in the pane, gated as it always was
on having a character selected.

Creating a character now lands on their STATS subtab. The sheet is in
the same tab as the form, so making one no longer means changing tabs -
but it does mean the sheet could be left on whichever subtab the LAST
character was read on, which for a brand new one is never right.

    Stats              level and abilities
    Description        species, body, features, drawbacks, languages
    Equipment          weapons, loot, general - in that order
    Skills & Talents   skills, proficiencies, talents and where each came from

Combat, spells and background are the obvious remaining two or three.
Every one is a button and a pane: `#sheet-tabs` runs on `showSub`, the
same helper the Characters tabs have used since 033.

**EQUIPMENT IS THREE DIVISIONS AND THE ORDER IS THE POINT.** What you
fight with first, because it is what you reach for under pressure; what
you are carrying out second, because it is what the session was for;
everything else after. One flat list sorted by nothing made a greatsword
and a blanket equally hard to find.

The grouping is the catalogue's own `kind` - the column the engine
already branches on for the one-armour rule - rather than a
classification invented on the screen. GENERAL IS DEFINED AS THE
LEFTOVERS rather than as a list of kinds, so a kind nobody has thought
of yet lands there instead of vanishing off the screen.

**PROFICIENCIES MOVED TO SKILLS & TALENTS.** What a person is trained
with is a fact about them rather than about what is in their pack, and
every weapon row already says `proficient` on its own - which was the
half that belonged beside the gear.

**A TALENT NAMES ITS SOURCE.** The Talents section lists the applied
species traits and the skills a people grants outright, each tagged with
where it came from - so a player can see why Athletics is ticked when
they never chose it. Drawbacks and the full prose stay on Description:
that tab is the "who you are" reading and this one is "what can you do",
which is why the same traits appear differently in each rather than
twice the same way. Class features join these when 055's deliberate
omission is filled in.

## A bug that had been on screen since 008

`[hidden]` did nothing to any element carrying a class that sets
`display`. The browser's own `[hidden]` rule is a UA default with almost
no specificity, and `.row{display:flex}` beats it.

The visible cost was the PROFICIENCY EDITOR, which has sat permanently
expanded under the trained line on every character sheet since it was
written - carrying `hidden` the whole time and honouring it never. It is
in the first screenshot of the app in this project's history and nobody
read it as a fault, because a form that is always open looks like a form.

Fixed globally - `[hidden]{display:none !important}` - rather than on the
one element, because every other `hidden` in the app was one `display`
rule away from the same thing.

## Where this run got to

Eight commits on this machine, migrations 055 to 059, on top of fifteen
pulled from the laptop (per-object techniques, initiative and the turn,
the encounter screen). The through-line is that a CHARACTER became a
thing the app can describe rather than a row with a
name on it.

**The arc.** `create_character` took a name and inserted a name. Level
defaulted to 1 and everything else to NULL, which meant no size, no hit
die, and NO HIT POINTS AT ALL - not zero, null, an empty space on the
sheet where a life goes. Snot and Unnamed stood that way for weeks and
nothing complained, because nothing was asking. Everything below came
out of pulling that thread.

    055  classes        the twelve, and the hit die a PC's hit points
                        actually come from. vitality::pc_hp beside
                        average_hp: two rules, not one with an exception
    056  species        a people, their modifiers, and an honest line
                        between what the engine applies and what a DM
                        must
    057  size           height in feet, playability, and ONE size ladder
                        where there had been four. The Unt'gar
    058  the body       height, weight, hair, skin, eyes; spoken and
                        written kept apart; a trait that can cost you
                        something
    059  short stride   the Unt'gar's 25 feet, said where a player will
                        look for it

Plus the numeric helper that started the run, the level-button ripple,
and the sheet split into four subtabs.

**Three bugs, and all three were one shape: a fact written down in more
than one place.**

    supabase::numeric   the ladder of numeric readers - seven of them,
                        three of which had been wrong, two of which
                        still carried comments saying so
    size.rs             the size ladder, written out FOUR times, in a
                        file whose own comment warned that a second
                        copy would eventually disagree
    rederive_hp_max     set_actor_level rippled and set_level did not,
                        so a player's maximum froze at creation. Garn
                        showed 12 at level 5

Each was fixed by making one place right and having everything else ask
it. That is now the house reflex and it is worth keeping.

**Two bugs that had been on screen the whole time and read as normal.**
A weight override emptied the entire Objects tab, because `Obj` is
deserialised by serde and the declared type IS the parser. And
`[hidden]` did nothing to any element with a class that sets `display`,
so the proficiency editor has sat permanently expanded on every
character sheet ever rendered - visible in the first screenshot in this
project's history, and read by nobody as a fault, because a form that is
always open looks like a form.

**What the campaign now has.** Two peoples seeded whole - the Unt'garoth
at 7-10 feet and the Unt'gar at 4-5 - out of a roster spanning a 2-foot
rodent people to a 25-foot Imiear. Garn is a level 5 Unt'garoth
Barbarian: STR 20, CON 14, 50 hit points, Large, carrying as Huge,
Athletics granted.

**The decisions waiting on Dave.**

  - THE JOTUN AND THE IMIEAR ARE BOTH HUGE. Eighteen feet and
    twenty-five, and not one number between them differs. A test in
    size.rs asserts it so it cannot be forgotten. The fix, if it is one,
    is to scale reach, space and carrying off `height_ft` and keep the
    category for the discrete rules.
  - TWO SPECIES AND ONE PEOPLE ARE UNSEEDED - Jotun, Imiear, and the
    rodent people, whose name is not known here. Heights only. A row
    with a name and six defaults is worse than no row, because it is
    pickable and gives a character nothing. The Fjell'gar landed in
    096 and the Ny'ook in 097.
  - A SOURCE THAT CONTRADICTS ITSELF IS RECORDED, NOT RESOLVED - and
    then resolved by Dave, which is how it is supposed to go. The
    Ny'ook document said "+2 on saving throws" twice and "advantage"
    once; 097 seeded neither and carried both readings, and 098 made
    it the +2 the day he chose.
  - A CIRCUMSTANCE CAN BE PART OF A REQUEST. `wis save vs spell` is
    the first one. Every other conditional trait stays a DM call
    because nothing can detect the condition - Stonecunning doubles
    on SOME stonework and no request says which - but the player
    knows at the moment they roll whether a spell is casting it, and
    the request has always been what they know. Four spellings are
    accepted; the roll KEY stays `wis_save`, because that is the
    vocabulary narrative_lines is written in and a key nobody seeded
    would cost a character their prose.
  - ENDURING MIGHT AND STONECUNNING both grant conditional expertise -
    doubled proficiency on SOME uses of one skill. Nothing can detect
    which use, so both are granted at ordinary proficiency with the
    doubling left to the table.

## How many swings - BUILT (061, class::attacks_at, spent::Budget)

**054 SAID THIS WOULD HAPPEN AND IT DID.** It counted actions, called a
second one "beyond one turn", and wrote down that "Extra Attack, haste,
action surge and a legendary action all make this true and legitimate,
and the engine knows about none of them."

Garn is a level 5 Barbarian. He is owed TWO attacks. Every second swing
he has ever taken was flagged as irregular by an app with no way to know
it was owed to him - and a warning that fires on correct play is worse
than no warning, because a DM learns to ignore it and then misses the
one that mattered.

**THE PROGRESSION IS DATA.** `classes.extra_attack_levels` is the list
of levels at which a class gains another attack, so the count is `1 +
how many you have reached`. Fighter {5,11,20} reads 1, 2, 3, 4 across
twenty levels; Barbarian, Paladin, Ranger and Monk are {5}; the other
seven are empty. On the row because 055 gave classes nullable tenancy so
a table could write its own - a match arm in Rust would make a homebrew
Fighter a code change.

The Bard is empty and WILL BE WRONG for a College of Swords bard, whose
Extra Attack comes from a subclass at 6. 055 has no subclasses. Written
down rather than discovered.

**THE ATTACK ACTION IS AN ACTION, and a pre-existing test caught me
getting that wrong.** The first version of the rule gave attacks and
other actions separate pools, which let a level 1 character swing and
then make a check inside one turn. `a_check_is_an_action_too` has
asserted otherwise since 054 and was right. Swinging N times costs ONE
action however large N is; swinging at all and then making a check costs
two, because the check needs the action the Attack already spent.

**WHAT AN ACTION COST.** `actions.cost` is stamped by trigger from
`key` - a swing costs an attack, everything else costs the action - and
83 existing rows were backfilled by the same rule, so history and future
agree rather than the backfill being a second opinion.

054 refused to classify 193 techniques and was right to; that refusal
does not apply here for two reasons. This is DERIVED from a column that
already exists rather than invented. And the techniques are all one
thing - checked, not assumed: every one of the 193 has dice and a weapon
mode, so there is no bonus-action technique to misclassify.

`bonus`, `reaction` and `free` are legal values that NOTHING WRITES. The
column admits them so the first one is a write rather than a migration,
and the screen deliberately shows no counters for them - an always-zero
"bonus 0/1" is a claim that a system exists. A HELD ATTACK is the one
Dave named with no home yet: readying is an action that becomes a
reaction on a trigger, so it needs both halves and something to hang on.

**ONE LOADER, STILL.** `Effective::attacks` joined the loader the laptop
built rather than a second per-character read appearing beside it. The
fourth request came free of a fifth: the first already reads
`characters`, so `class_key` and `level` cost nothing there, and only
the twelve-row class catalogue is new - skipped entirely when nobody in
the fight has a class, which is a roster of monsters.

**THE SCREEN PRINTS A PAIR AND DOES NO ARITHMETIC.** "1/2" where it used
to say a bare count, in the order list, the strip above it, the roster
tag and the card head. The caret marking whose go it is sits in the
MARKUP rather than only in a border, so the row still says so when read
aloud or at phone width. And the card's button names the next creature -
"End turn to Goblin Scout", or "(round 2)" when the order is about to
wrap - which 051 had already argued for the bar and never done here.

STILL NOT A GATE. 051 decided the order informs and refuses nothing, and
a budget is the same kind of thing: it says what is owed so two swings
read as two of two. Haste, an action surge and a legendary action remain
outside what the engine knows, and remain legitimate.

Double-tested as asked. Unit: 504 passing, including a Fighter walked
level by level, four swings inside one action, and a stated cost beating
the key. Live and rolled back: three actions inserted for Garn with no
cost stated came back stamped `attack, attack, action` with 060's round
still applied, and 061 computes his owed attacks as 2. Rig: the order
renders `1/2` and a warned `2/1`, the caret follows the turn, and the
button names the next creature in both the ordinary and the wrapping
case.

## The other three slots - BUILT (062, spend_slot / clear_slot)

061 made `bonus`, `reaction` and `free` legal and left them empty, and
said why: "the column admits them so the first one is a write rather
than a migration, and the screen does NOT show counters for slots
nothing can fill." This is that write.

**A SLOT IS SPENT BY WRITING AN ACTION WITH NO DICE.** The alternative
was a per-turn state table, and it would have been a second account of
the same round. An action already gets its round stamped (054, 060),
already records whose turn it was taken in, already appears in the log,
and already deletes cleanly - which is exactly what un-ticking a box
has to do. The table already permitted the shape: every roll column
lives on `rolls`, not here.

**THE KEY AND THE COST AGREE BY CONSTRUCTION.** 061's trigger read
`attack` from the key and called everything else an action, so a marker
keyed `bonus` would have been stamped `action` unless the caller passed
the cost too - two things to keep in step. The trigger now recognises a
cost word used as a key, and a stated cost is still kept for the
attack technique that one day costs a bonus action.

**EACH SLOT HAS ITS OWN TALLY.** Before 062 everything that was not a
swing landed in `other`, which was right while nothing could write the
other three and would have made a Rogue's Cunning Action read as a
spent turn the moment something could. `an_attack_and_a_bonus_action_is
_one_ordinary_turn` is the test that pins it.

**THE TICKS.** Three boxes on the fight card for the creature being
acted from, and `b` / `r` / `f` beside the swing tally in the order
list so the card is not the only place they can be seen. The box
character is in the TEXT rather than only in a colour - the same call
061 made for the turn caret, and for the same reason: a state carried
only by colour says nothing read aloud or at phone width.

ONLY WHILE THE FIGHT IS RUNNING. A round of 0 means "not started", and
060 keeps a roundless action out of the count - so a tick in a
not-yet-begun fight would write a row nothing would ever show.
Verified: the boxes are absent at round 0 and the button reads
"Begin -> Garn", the top of the order.

NOTHING IS REFUSED, including ticking twice. 051 decided the order
informs and never refuses, and a slot is the same: a DM granting a
second bonus action is an ordinary Tuesday, and an app that argues
about it is one they fight. A second tick reads "Reaction x2" with a
red rule and the count says over budget.

**A REACTION IS ONCE PER ROUND AND THE OTHERS ARE ONCE PER TURN.** 5e
refreshes a reaction at the start of your turn, so a creature that acts
once per round is the same either way - which every creature here does.
The counting is per round, it is right for every case the app can
currently produce, and it wants revisiting the day something takes two
turns in one round. Written down rather than discovered then.

**THE HELD ATTACK still has no home**, and it is the last of the five
Dave named. Readying is an action that BECOMES a reaction on a trigger,
so it needs both halves and something to fire it. What 062 gives it is
the reaction slot to land in when it is built.

Double-tested. Unit: 509, including a full legal turn of two swings, a
bonus, a reaction and a free interaction reading as not over budget,
and Extra Attack buying no extra reactions. Live and rolled back: three
markers written with only `key` set came back stamped bonus/reaction/
free at round 1, and clearing the reaction removed exactly that one.
Rig: the ticks render and toggle, an unticked box calls `spend_slot`
and a ticked one calls `clear_slot` with the right cost, a doubled slot
shows "x2" with its red rule, and round 0 shows no boxes at all.

## Holding a place - BUILT (063, initiative::place_holds)

**062 GUESSED WRONG ABOUT WHAT A HELD ATTACK WAS.** It assumed 5e's
Ready: an action converted into a reaction, fired by a trigger somebody
describes in prose and the app has to watch for. Dave corrected it - a
held action here DECLARES A POSITION. Go after the next one, go after
that character, go at the end of the round.

That is a statement about the ORDER, which this app has, rather than
about an event, which it does not. The trigger version needs a watcher
and a vocabulary of conditions; this needs two columns and a sort.

**THE ROLL IS STILL THE RECORD.** A hold does not rewrite `initiative`.
011 made that column what somebody rolled and this lays a declaration
over it - release the hold and they are back where the dice put them,
with nothing to restore because nothing was overwritten.

**A HOLD LASTS ONE ROUND.** "End of round" only means anything inside a
round, so that is the life of the whole declaration: advancing into a
new round clears every hold, and so does a reset. The alternative - a
hold that persists until cancelled - means a creature who held in round
1 quietly acting last in rounds 2, 3 and 4 because nobody remembered.
That is the kind of state that makes a tool untrustworthy at the table.

The wipe happens BEFORE the pointer moves, not after. If it fails the
turn has not advanced and pressing the button again is harmless; the
other way round leaves the fight on round 2 carrying round 1's
declarations, which is the state nobody could explain.

**THE PLACEMENT CANNOT LOOP AND CANNOT LOSE ANYBODY**, which are the
two things worth guaranteeing about a sort that takes instructions.
Everybody not holding keeps their rolled place; the end-of-round
holders go last; then the ones naming a creature or a slot are inserted
repeatedly until a pass places nobody new. A holding after B while B
holds after A is a declaration with no answer - each pass places
neither, the loop stops, and both fall to the end in rolled order. Same
for a hold naming somebody who has left: 063's foreign key nulls the
target and an unresolvable target is treated exactly like an unplaceable
one.

`after_next` means ONE PLACE LATER, and the neighbour it waits for is
somebody who is not also holding - waiting for a creature who is
themselves waiting is not what the words mean.

**ONE THING IS REFUSED AND THE REST ARE RESOLVED.** Waiting for
yourself is a check constraint, because it is not a declaration anybody
could mean. Everything else follows 051: naming somebody who has
already acted is legal and means acting sooner than the dice said,
which a DM may well want; naming somebody gone resolves to the end of
the round and the screen says so, because a row out of its rolled place
with no explanation reads as a broken sort.

**ON SCREEN**: two toggles and a picker on the fight card, the picker
excluding the creature themselves. A held creature carries a pause
glyph in the order and their initiative is struck through - the glyph
is in the MARKUP rather than only a colour, the third time that call
has been made, after 061's turn caret and 062's tick boxes.

Double-tested. Unit: 525, including the cycle, a chain, a target that
has left, a fight where everybody holds, `after_next` at the bottom of
the order, and the placement being identical across five repaints.
Live and rolled back: all four guards refuse - holding for yourself, a
mode nobody defined, `end_of_round` carrying a target, and naming a
creature in a different fight - and the round wipe clears every row.
Rig: Garn at initiative 16 renders below a Goblin Scout at 5 with the
pause glyph and "holding - after Goblin Scout", the picker is set to
the creature he named and does not offer him, and the three controls
send `end_of_round`, `after_actor` with the id, and `release_hold` when
the picker is cleared.

**THAT IS ALL FIVE.** Actions, bonus actions, instant effects, held
attacks and additional attacks - 061 did the swings and the action,
062 the other three slots, 063 the hold. What the economy still has no
model for is haste, an action surge and a legendary action, all of
which read as over budget and all of which are legitimate; the flag
says "look at this" and has never claimed more.

## NPCs have a class - BUILT (064, 065)

055 gave a CHARACTER a class and stopped at the player side. Every
monster in the game had `class_key` NULL, so `Effective::attacks` handed
them one swing and `class::attacks_at` was never asked. Dave's rule:
most combat types are fighters or rogues, the shop keeper is a rogue,
the bar keeper is a bard.

**THERE WAS ALREADY A `class` AND IT IS PROSE.** `npcs.class` has held
"Fighter" on the Goblin Fighter since 022 and NULL on the Goblin, and
nothing has ever read it. `class_key` is the reference; the text stays
as the DESCRIPTOR, because a statblock may want to say "Chieftain"
where no catalogue class fits and losing that would lose something a DM
wrote. The key is the rule, the text is the label, and the form now
takes both - verified through the rig: `class: "Brute"` and
`classKey: "fighter"` in one payload.

**THE LINE THAT MATTERS: A CLASS DOES NOT CHANGE A MONSTER'S HIT
POINTS.** 029 settled that a monster's maximum is level times the die
their SIZE gives, because the Monster Manual writes 7 (2d6) and that is
what makes "set level" a button rather than a rewrite. 061's `pc_hp` is
the other rule and it is not this one.

`rederive_hp_max` recomputes from class for anything that has one, and
before 064 no NPC did. Left alone, the Goblin Scout would have gone
from 28 hit points to 43 the first time anybody touched their level,
silently. The guard went in with the sweep: an NPC returns early and
keeps 029's rule. Verified live - the probe printed "scout keeps 28 (pc
rule would say 43)".

**WHAT A CLASS GIVES A MONSTER** is an attack count and a name for what
they are. Nothing else: 055 deliberately has no class features, so a
Rogue goblin gets no Sneak Attack and no Expertise.

**THE SWEEP.** Twelve instances and two statblocks, none left
unclassed. Goblin Fighters are Fighters because their own statblock
said so. Everything else goblin is a ROGUE: a 5e goblin's signature is
Nimble Escape - Disengage or Hide as a bonus action - which is Cunning
Action wearing another name, and a skirmisher is the closer read than a
line fighter. Dave's rule allows either.

EVERY INSTANCE STILL COMES OUT AT ONE SWING, because a Rogue gains none
and a level 1 Fighter has not reached five. Nothing about the fights on
the table changed, which is the right way for a sweep to land.

**PROFICIENCIES WERE FILLED, NEVER OVERWRITTEN.** An empty list is
028's fault and reads as proficient with nothing; three Goblin Fighters
had been carrying one and now hold `{sim,mar}` and `{lgt,med,hvy,shl}`.
Goblin Fighter 0004 kept the `{shortsword}` somebody authored.

**065 IS THE HALF THAT WOULD HAVE BEEN MISSED.** `instantiate_npc`
copies ten columns from the statblock and could not copy the eleventh,
so the THIRTEENTH goblin would have come out classless - the same
half-a-feature as `objects.attuned` since 008 and `characters.markup`
since 040. Verified live and rolled back: a fresh goblin comes out
`rogue`, 7 hit points, level 2, `{sim}` intact.

**ONE THING LEFT FOR DAVE.** Merchant 1 is the shop keeper and Dave's
rule says a shop keeper is a rogue - but `is_npc` is FALSE on that row,
because he was made a PC to test the shop UI. The sweep took the flag
at its word and left him alone. Classing him is one update; it would
also move his maximum from 22 to 28, since he is a character and
`rederive_hp_max` applies to characters. His call.

## Rolling a character up - BUILT (066, generation.rs)

Seven scores of 3d6 with every 1 rerolled, six assigned by elimination,
locked, and written when the character is created. The spare is the
point: the worst of the seven need not be lived with.

**WHAT "REROLL 1s" WAS TAKEN TO MEAN.** A 1 is rerolled, and a reroll
that is also a 1 is rerolled again - no 1 survives. Each die is
uniform over 2..=6, a score runs 6 to 18 with a mean of 12, and 3, 4
and 5 are impossible. The other reading - reroll each 1 ONCE and keep
what comes back - leaves 1s on the table and means about 11.75. Both
are in use at real tables; this is the commoner phrasing and the
kinder. If it is the wrong one, `REROLL_BELOW` and the loop in
`score_with` are the whole change.

**IN ITS OWN FILE** because `dice.rs` was already at 894 lines, past
the 800 ceiling ARCHITECTURE.md sets, and because this is a different
subject: dice.rs resolves a formula somebody typed, generation.rs rolls
a character up and will grow - starting coin, starting kit, a point-buy
alternative.

It takes a `Roller` rather than reaching for the RNG, which is what
makes the rule testable: a fixed sequence in, an exact spread out. The
reroll loop is capped at twenty per die - not for the RNG, which clears
a 1 with probability 1, but so a misbehaving Roller fails a test rather
than hanging the suite.

**THE PICKS KEY ON INDEX, NOT VALUE**, and that is the one thing in the
screen worth saying. A spread of 16, 14, 14, 11, 9, 13, 7 has two
fourteens, and they are two separate things to spend. Keyed on the
value, assigning one would have removed both. Proved in the rig: after
DEX took the first 14, CON could still see the second.

**THE RULE IS NOT ONLY IN THE DROPDOWN.** The pickers make a duplicate
impossible by construction, and `generation::complete` says the same
thing in Rust - six codes, each exactly once, each in range - because
the next screen will not have a dropdown. It does NOT check a score
came from the spread: a DM may say a number, and 051's rule holds.

**THE ORDER OF THE WRITE IS THE WHOLE OF IT.**
`seed_character_abilities` writes ten across the board on insert, so
the picks land BEFORE hit points are worked out. Reading Constitution
first would give every character the hit points of a 10 - a Barbarian
who rolled 16 would be quietly short for their whole career. Verified
live and rolled back: an Unt'garoth Barbarian with a rolled CON 14
comes out at 14 hit points (14 +1 species = 15, +2, on a d12); before
the picks landed the same character would have shipped at 12.

**THE SHEET'S ABILITY BOXES ARE READ-ONLY NOW**, with an "edit scores"
toggle that opens all six at once. Scores are rolled at creation and
then mostly left alone, and an editable box invites a stray keystroke
into the one number every modifier on the sheet derives from.

**CREATING A CHARACTER OPENS THEIR SHEET.** Being left on the form was
the gap - the rest of a character is edited on the sheet, so that is
where making one should end. The form clears behind it.

Double-tested. Unit: 538, including the reroll, the impossible 3/4/5,
a Roller that only gives 1s terminating, roll order being preserved
rather than sorted, and five ways an assignment can be incomplete.
Rig: the pool struck through as it is spent, elimination across a
duplicate value, the lock enabling only on the sixth pick and
disabling the pickers, and the payload carrying all six pairs with the
7 discarded. One bug found and fixed in the rig - the Lock button kept
its "Abilities locked" label over the empty pool of a fresh form.

## A character can be renamed - BUILT (072, naming.rs)

The sheet could set a level and could not set a name. An object had
`rename_object` and an encounter actor had `rename_actor`; the one name
a player types first was fixed at creation, so a typo at the form was
permanent. A name box now leads the stats row with the level beside it.

**The part that is not one update.** A character carries two names:
`name`, and `token_name`, which is what the roster, the initiative strip
and every roll card actually print. Moving `name` alone changes the
sheet heading and nothing else - the same half-fix `edit_object` had.
Overwriting both is also wrong, because a short name that differs from
the long one is something somebody chose.

So naming.rs decides, and the rule is narrow:

| old name | old short | new name | short becomes |
|---|---|---|---|
| Test PC 1 | NULL | Garn | NULL |
| Snot | Snot | Grisk | **Grisk** - it was a copy, so it follows |
| Rodnar Shieldcrest | Rodnar | Rodnar Oathbreaker | **Rodnar** - it was a decision, so it stays |

That middle row is the NPC case: `instantiate_npc` writes the same text
into both columns, so without it a goblin renamed would have kept
answering to Snot everywhere except its own sheet.

**NOT RETROSPECTIVE.** Rolls and actions keep the name that was true
when they happened (001), and an actor already standing in an encounter
keeps its own label. `rename_actor` is still the tool for that,
deliberately separate, because a disguise is a real thing to want.

## Multiclassing - BUILT (073, multiclass.rs)

055 gave a character `class_key` and `level`: one class, and that level
column was doing two jobs at once - the class's level and the
character's. For a single-classed character they are the same number,
which is why it worked and why it would not stretch. A Fighter 5 /
Rogue 3 has three levels at once: 5, 3, and the 8 that buys their
proficiency bonus.

`character_classes` holds one row per class. The Stats row shows the
leading class between the name and the level, and the level beside a
class is THAT CLASS'S level; additional classes get a row each
underneath with their own level and their own Set level.

**Three rules that a lot of character sheets get wrong:**

- The proficiency bonus is off the TOTAL. Fighter 5 / Rogue 3 gets a
  level 8's +3.
- The first level's whole hit die belongs to the STARTING class and is
  paid once in a career. Taking a second class at level 6 does not hand
  out another maximum. This is what `added_at` is for - it is the
  starting-class rule and the tie-break for which class leads, not a
  timestamp for curiosity.
- **Extra Attack DOES NOT STACK.** Fighter 5 / Ranger 5 swings twice,
  not three times. `multiclass::attacks` takes the best.

A test pins multiclass HP against 061's `pc_hp` for every die, level and
Constitution in range, because a character who takes a second class and
drops it again has to land back on the number they started with.

**`characters.level` and `class_key` STAY, and a trigger keeps them.**
A monster has a level and no class, and every screen reads both - making
them all sum a second table would be a dozen new round trips to answer a
question the row already answers. But nothing in the app writes them for
a classed character any more: `sync_character_level` does, from the class
rows, and `set_level` refuses outright for anybody holding any. There is
no honest way to spread "make them level 7" across a Fighter 5 / Rogue 3,
and a second writer is the two-places-disagree fault this codebase has
now been bitten by three times. A trigger cannot be forgotten.

Verified on a rolled-back probe: +rogue 3 reads level 7 led by fighter,
rogue raised to 6 moves the lead, dropping it returns 4, and dropping the
last class leaves the level alone rather than making anybody level zero.

**Two design calls that are Dave's to overturn:** blanking the class
dropdown and pressing Set level drops that class (it is the only route
back to classless, and the extra rows have an explicit remove); and which
class leads is computed - most levels, ties to whichever was taken first -
rather than chosen. Making the primary a DM decision is a column and an
afternoon.

**NOT BUILT: multiclass spell slots.** Half and third casters add on a
separate progression table, and nothing in this engine casts anything
yet. It gets its own file when it does, not a fourth rule in this one.

## The bard, stage 1 - BUILT (074, 075, 076, karma.rs)

Instruments exist, a bard has a Karma rating, and a performance resolves
end to end against a chosen audience. On Skills & Talents: the Karma
block with its working, an instrument picker, an audience picker, and
the odds shown BEFORE you commit to them.

**What 5e actually has, so the line is clear.** Instruments are tools
and a bard gets three of their choice; an instrument may be a
spellcasting focus (that is the "channel or totem" half, and it is RAW);
Bardic Inspiration gives ONE ally a die for 10 minutes and does not
stack. Performance is a d20 Charisma check. **There is no percentile
anywhere in 5e and no "draft a song" mechanic at all** - the % roll, the
1-hour duration and the party-wide target are Dave's, marked as his in
the code.

**074** adds `instrument` as a seventh `items.kind`. The check
constraint refused the first insert, which is the constraint doing its
job - widening it deliberately is the whole difference between a new
category and a typo. Nine of them, stats deliberately flat (1-5 lb,
1-35 gp) because no rule anywhere distinguishes a drum from a viol. All
carry `foc`, which READS NOWHERE because nothing casts; the catalogue
should not lie about what a lute is.

**075** adds `characters.tool_profs` beside `weapon_profs` and
`armor_profs`, through the same `equipment::is_proficient`. Its
catch-all said "nothing else grants or needs proficiency", which stopped
being true the moment there was a lute. `classes.tool_choices` says a
bard is owed three; nothing enforces it, exactly as `skill_choices` has
not since 055.

**076 is the rule.** Karma is a property of the CLASS -
`classes.karma_skills`, bard is `{ins,prf}` - so the second class to get
one is a row, not a release. Expertise counts, which means a late-career
bard sits pinned at the chart's ceiling: that is intended, the ceiling
IS the reward.

The HOPPER chart is:

```text
    target = 50 + 2 * (audience - karma)      roll OVER it, held 01..99
```

which makes the whole 676-cell table ONE number - the gap, -25 to +25 -
and the diagonal 50 everywhere along it. An even match is a coin flip
wherever on the chart it happens. It never reaches certain at either
end: beating 01 still fails on a natural 1, beating 99 still comes off
on a 00. Twenty-six tests, including all five anchors read off Dave's
printed chart.

**IT RAN THE OTHER WAY FIRST, AND THE FIRST LIVE PERFORMANCE IS WHAT
CAUGHT IT.** Falon rolled 26 against a printed 38 and the screen said
"made it by 12". Dave said that is a miss: his chart is a difficulty to
clear, not an allowance to stay inside.

Both halves had to move, which is the part worth remembering. Flipping
only the comparison leaves `karma - audience` raising the number, so a
master in a friendly room would need to beat 100 and the better you got
the worse you would do. Mirroring the axes as well keeps low good and
leaves the ODDS for every pairing exactly where they were - a cell that
printed 38 and meant 38% now prints 62 and still means 38%.

The ceiling moved with it: 100 printed as `00` was a roll-under
necessity and nothing prints `00` any longer. The hundred is a ROLL
now, the one that beats the worst corner.

**The audience ladder is DATA, not code**, because the spacing is a
tuning guess and retuning a guess should be one UPDATE and no rebuild:

| Audience | rating | | Audience | rating |
|---|---|---|---|---|
| Participating | 0 | | Busy | 13 |
| Watching | 3 | | Distracted | 18 |
| Some interest | 5 | | Hostile | 24 |
| **Neutral** | **8** | | | |

Goodwill is compressed and hostility spread on purpose - a friendly room
helps less than a hostile one hurts. `audiences` is tenanted like every
other catalogue, so a game can insert its own "Royal Court".

**A bug that only live data would have found.** Falon was already a
Fighter 4 / Bard 1 in Dave's running app, and `derive_karma` found the
LEADING class and then asked whether it had a formula. Fighters have
none, so it stopped there - a bard with a sword had no Karma at all. The
filter belongs inside the search. Unit tests alone would have shipped
it; reading the rows before committing caught it.

**NOT APPLIED, deliberately:** whether playing an instrument you were
never taught should cost you. `proficient` comes back on the result and
the screen says "untrained", but it changes no number. The chart has
only two axes, so folding in a third thing means either docking Karma or
treating the room as harsher, and both are guesses. Dave decides.

**STAGE 2 AND 3 ARE NOT BUILT.** Stage 2 is `songs` - a drafted song as
a record, the way a roll is one. Stage 3 is `effects`, and it is the
only hard part: **there is no conditions table and no clock between
encounters.** The engine counts rounds inside a fight and has no concept
of time outside one, so "inspires for 1 hour, does not stack" has
nowhere to be measured or enforced. That is a design decision before it
is a coding one. Estimated at ~8 hours once the time model is settled,
and it pays off across everything else queued - haste, action surge,
legendary actions, poison, rage, exhaustion all want the same table.

## Game time - BUILT (092, 093, clock.rs, uses.rs)

The engine counted rounds inside a fight and had no idea what time it
was outside one. That blocked every timed effect - the bard's song,
concentration, exhaustion, and the rest cycle that brings Action Surge
back.

**ONE COUNTER, IN SIX-SECOND TICKS**, on `games.tick`. A round IS six
seconds, so every 5e duration is a whole number of them and there is no
second time system to keep in step:

```text
    1 round                              1
    1 minute                            10
    10 minutes  (Bardic Inspiration)   100
    1 hour      (short rest, attune)   600
    8 hours     (long rest)          4,800
    24 hours    (the rest limit)    14,400
```

An effect is not "an hour", it is `expires_at = now + HOUR`, and expiry
is one integer comparison that reads the same in a fight and on the
road. **Combat time and travel time stopped being different systems.**

GAME TIME, NEVER WALL TIME. Nothing reads `now()`. A session that breaks
for pizza has not aged anybody.

ONE CLOCK PER GAME. A party shares a timeline; a clock each would mean
reconciling them the moment somebody scouted ahead. The 24-hour long
rest limit is per CHARACTER though - `characters.last_long_rest` - because
5e's rule is on the creature, and a party splitting its watch does not
all sleep at once.

**THE DM MOVES IT.** Explicit jumps - 10 minutes, 1 hour, 4, 8, 24 - on
both Play and Run, because outside a fight nothing can know how long
anything took and an engine that inferred it would be confidently wrong.

**Uses and recharge.** `class_features.uses` is an EXPRESSION, because
almost none of them are a number: Action Surge is `1@2,2@17`, Ki is
`level`, Bardic Inspiration is `cha_mod`. Four forms cover nearly
everything - see uses.rs. The level is always THE CLASS'S, so a
Fighter 4 / Bard 1 gets one Action Surge rather than the two a level 5
might suggest.

`recharge` is short / long / day / dawn, and `Rest::restores` holds the
asymmetry: **a long rest gives back everything a short one would, and
not the reverse.** Verified on a rolled-back probe - a short rest cleared
Action Surge and Second Wind and left Indomitable spent; the long rest
cleared all three.

**Hit dice** are per class on `character_classes.hit_dice_spent`, because
the die is the class's - a Fighter 4 / Bard 1 spends a d10 or a d8 and
they are not interchangeable. A long rest returns half the total,
minimum one (5e's own minimum, and the reason `dice_back` is a function:
half of one is zero).

**A long rest heals by RECORDING, not erasing.** Current HP is `hp_max`
plus the sum of the deltas, so wiping `hp_events` would also reach full
and would throw away every wound ever taken. 001's rule holds: a night's
sleep is one more event.

**NOT BUILT YET, and this is what the clock was for:** the `effects`
table. `clock::expired` and `clock::remaining` are written, tested and
carry `#[allow(dead_code)]` with that reason - they are the API the
effects table will ask. "1 hour, does not stack" is now measurable; it
still needs somewhere to live.

## Effects - BUILT (094, effects.rs)

The last piece 092's clock was built for, and the bard's Stage 3.
"Inspires compatriots for 1 hour, max does not stack" is two rules - a
duration and a stacking rule - and before the clock there was nowhere to
measure the first and nowhere to enforce the second.

**AN EFFECT IS A ROW WITH A DEADLINE**, in `games.tick`. Expiry is one
integer comparison, so an effect behaves identically in a fight and on
the road and nothing sweeps a table to notice an hour has gone.

**NOTHING DELETES AN EXPIRED EFFECT.** It is simply no longer active and
the row stays as a record of what was true - the same contract as a
roll. `ended_at` is for one cut short, which is a different fact from
one that lapsed. There is no DELETE policy at all, so through PostgREST
nothing can remove one.

**FOUR STACKING BEHAVIOURS**, and which applies is a property of the
EFFECT rather than of the engine:

| | |
|---|---|
| `replace` | the newcomer wins - a fresh song restarts the hour |
| `highest` | the better survives, and **a tie goes to what is already running** |
| `stack` | both run - two poisons, two wounds |
| `refuse` | a second is turned away while the first holds |

`highest` is the subtle one: **a weaker version of something already
running is NOTHING rather than an error.** Refusing it would let a bard
who sings badly undo their own good song.

Only the SAME KEY on the SAME CHARACTER is ever in the way. Deciding a
blessing and a bard's song are "the same bonus" is a rule about those
two things and not one 5e makes.

**The bard's song now lands.** A successful performance applies
`inspired` to every player character for an hour, `highest`, with the
die off the performer's Karma - d6 under 8, d8 at 8, d10 at 14, d12 at
20. That scale is DAVE'S, not 5e's: Bardic Inspiration climbs with bard
level and this is a different feature on the HOPPER chart's axis, so one
number decides both how likely the song was and what it is worth. The
result names who it reached, and leaves off anybody already carrying
better.

Effects show as chips on Play and Run, soonest to end first, each with
the time left and an x to end it early.

**WHAT IS STILL NOT BUILT:** the `songs` table - Stage 2, skipped. A
drafted song is not yet a record you can name and keep; the performance
rolls and applies and nothing persists the song itself. Also nothing
READS an effect yet: `inspired` sits on a character and no roll adds the
die. That is the next wiring, and it wants the modifier pipeline 080
started.

## Enchantment, as one vocabulary - BUILT (100, 114, 115, grants.rs)

A column per effect - `attack_bonus`, `ac_bonus`, `str_bonus` - is a
migration every time somebody invents an item, and six readers that each
know a different subset. A **grant** says what it touches, how, and by
how much, and every consumer asks `grants.rs` the same question:

```json
{"target": "attack", "mode": "add", "value": 1, "source": "a +1 sword"}
{"target": "str",    "mode": "set", "value": 19}
{"target": "save",   "mode": "add", "dice": "1d4", "source": "Bless"}
```

`add` is signed and cumulative; `set` is a FLOOR, which is how 5e words
every item that uses it ("your Strength is 19 unless it is already
higher"). A set that LOWERS is not expressible on purpose - that is a
curse, and a curse is a negative `add`.

114 added `dice`, because Bless is +1d4 and not +2. A die never collapses
into a number: `effect_grants` appends it to the formula so the roll
shows `1d20+7+1d4` and the card says what it rolled.

`known_target` refuses a target nothing reads. A grant written against
`armour` parses perfectly, stores perfectly, and does nothing for ever -
the item simply is not magic and nobody can see why. **That is this
codebase's named defect class and the cheapest place to stop it is the
write.**

115 is the correction migration: I had invented two spell grants.
Protection from Evil and Good does not add 1d4, and Resistance is one
save rather than every save. Corrected in a new migration rather than by
editing 114, because 114 had been applied.

## Spells and prayers - BUILT (101-112, prayers.rs, spellcast.rs)

A spell catalogue rather than one character's list - 006 had ported
spells with a single character's numbers baked in (spell_atk +7, DC 15),
flagged BAKED in the column comments. 101 replaced that with a real
catalogue; 102-104 seeded the whole cleric list; 106 and 107 gave a
character prepared prayers and slots that can actually be spent.

`prayers.rs` is the 5e arithmetic - prepared maximum is WIS + cleric
level, the full-caster slot table, save DC and attack bonus.
`spellcast.rs` reads a spell's own text to decide what it COSTS (action,
bonus, reaction, or too long to cast in a fight) and how long it LASTS,
so a 1-minute Bless becomes an effect and an instantaneous Cure Wounds
does not.

A cast spends a slot, writes an action row, lands an effect on its
target and ends whatever the caster was concentrating on. That last
clause is 5e's most-forgotten rule and it is enforced on the CASTER
rather than the target, so Bless on three allies is one concentration.

## Damage resistance - BUILT (116, 117, 118, resist.rs)

`species.damage_resistances` had existed since 056 and the sheet printed
it followed by "(DM applies)" - an honest label for a thing that did
nothing. A resistance is a grant now: `resist.fire`, `immune.poison`,
`vulnerable.cold`, which bought items, spells and class features at once.

Immunity is not more resistance - half of a large number still kills and
none of it never does - so the three degrees are a `Degree` and not a
scale. 5e's stacking rule is that there is **no** stacking, and both
sources are still shown, because hiding the redundant one hides the fact
that ending Rage will not lift the species half.

A feature with no `uses` grants passively; one with uses grants only
while its effect runs. Getting that backwards would make every barbarian
permanently resistant to bludgeoning, piercing and slashing.

Protection from Energy asks a question - "choose one of acid, cold,
fire, lightning, or thunder" is one grant naming five types, settled at
the cast and refused rather than guessed.

Applied where damage lands, after the dice and after the crit. The roll
row keeps what it rolled; the hit point event carries the reduced number
and the log says which resistance did it.

117 retired the trait prose that said "there is no resistance system
yet, so halve it at the table". **An instruction left standing after the
feature lands is worse than the missing feature was, because somebody
follows it.**

## Saves - BUILT (118, 119, Sheet::save_line)

Two faults, one cause.

The Ny'ook's +2 against spells was real, tested, and unreachable: it
applied only when the request string ended in " vs spell", and the roll
box's own placeholder offered "wis save", which is the spelling that
does not get it. It is a checkbox beside Adv/Dis now - a circumstance is
a control, not an incantation - and it writes the suffix the engine
always read, so the vocabulary did not change.

And the ability rows carried a proficiency checkbox and no number at
all, so a Constitution save was invisible until somebody typed it into
the roll box. `Sheet::save_line` owns the arithmetic; `resolve_request`
is a caller. The row shows every modifier with its name on it, and
"saving throw" is a button that rolls it. A DC prompt opens below -
`state.scriptedSave` already short-circuits it, so a trap that knows its
own difficulty is a line of wiring rather than a rewrite.

## The migrations that were missing - RECOVERED (101-107)

`supabase/migrations/` is the record of how this schema was built and
for a while it was an incomplete one. Seven files had been applied to
the live project and never committed - 101, the spell catalogue, and
102 to 107, the whole cleric list with prepared prayers and slots.

RECOVERED FROM THE DATABASE'S OWN RECORD, not retyped.
`supabase_migrations.schema_migrations` keeps the statements it ran,
headers and all, so these are the files AS APPLIED. Every one was
MD5-verified against that record rather than read over:

```
101  3,331 chars   5b84f3c9205cb5d4a12598abd81e0b2b
102 15,781 chars   0036c54778715165e9cc6fd2518d0eeb
103 16,224 chars   ff6c9bfc1631396320c3787bf1dd0c2f
104 11,027 chars   5a0f3b599134a4a6004daeee327518b4
105  1,049 chars   f4607ce04066d471bd0aec7162957dd9
106  4,066 chars   a969beedd8f5bfdf66d2866677eed8c5
107  3,490 chars   6e335aada525e0314c8d42a23f53ea5b
```

AND THE OTHER NINETEEN GAPS TURNED OUT TO BE NOTHING. The first pass at
this called all 26 of them missing migrations, which was wrong and got
as far as the README before it was checked. Every change in this project
gets a number; only a change that touches the schema gets a migration
file. The comments say so where the numbers landed - 070 "taught the
sheet to show it" and 071 filled the column; 099 is an ability cap in
`species.rs`; 111 and 112 gave casting a target in JavaScript and Rust.
Nineteen numbers, no schema, nothing missing.

THE LESSON IS THE HABIT, not the recovery. Write the migration file in
the same breath as applying it. `tools/recover_migrations.py` is the
backstop: it writes any migration the database has a record of and the
folder does not, MD5-checks each one, and names the numbers it cannot
help with.

## A double-click was a second cast - FIXED (120)

Found by auditing the prayer subsystem rather than by anything going
wrong at the table, which is the only reason it is in this file and not
in a bug report.

TWO HALVES, AND THE BUTTON WAS THE SMALLER ONE.

**The button.** `guarded()` has existed since the death-save work and
its own comment says why: "a second dispatch is not a harmless
duplicate - it is a second swing nobody took". It took a SELECTOR, so
it reached the ten buttons that exist in `index.html` and none of the
ones built per row. Eight writes were unguarded - cast, prepare,
forget, spend a slot, give one back, roll a save, put a thing in a
container, give a feature use back. It takes an element now, and
`guarded(selector, fn)` is that function with a lookup in front, so
there is one mechanism rather than two that can drift.

**The slot.** The half that actually corrupted state, and a button
guard only hides it:

```text
let spent = load_slots(..)?;            // both casts read 0
may_spend_slot(level, &spent, want)?;   // both pass the check
write_slot(.., want, spent[i] + 1)?;    // both write 1
```

Two castings, two effects on the target, two action rows - and ONE
slot spent. `move_slot` is compare-and-set instead: the PATCH carries
`spent=eq.<what we read>`, so a write that lost the race matches no row
at all, and PostgREST handing back an empty array IS the collision,
reported rather than silently applied. No row yet is not a collision -
the first spend of a level falls through to an insert, and a second one
racing it loses on the primary key, which is the same refusal by
another route.

VERIFIED BOTH ENDS. A rolled-back probe on the live database:
`winner=1 rows, loser=0 rows, final spent=4` - the stale write matched
nothing rather than overwriting. And in the stub rig, three presses
inside thirty milliseconds against a 400ms cast produced ONE
`cast_prayer`, the button disabled for the flight and enabled after,
and a handler that throws still leaves its button usable.

AND THE SAME FAULT ONE FUNCTION OVER. `character_uses` had the identical
shape - `spend_use` read a count, checked it, and upserted an absolute
number - so a double-click on Action Surge granted two and counted one.
Its button was unguarded too. Both halves fixed the same way, with the
conditional PATCH extracted to `supabase::rest_update_if` so there is
one copy rather than two.

TWO THINGS FOUND ON THE WAY. The hold/release button in the DM panel
disabled itself and never re-enabled, leaning on a repaint to replace
it - a repaint that did not happen left a dead control on screen. And
`give_item` carried its own correct copy of `guard`, finally and all.
Both now go through the one mechanism; the first was a bug and the
second was a second place to get it right.

THE RULE THIS LEAVES: **a button that writes goes through `guard`, and
a counter that is read before it is written moves by compare-and-set
rather than by an absolute number.** Sixteen call sites now, and the
audit that found this is a dozen lines of Python worth re-running after
any panel grows a button.

## A monster can say what it resists - BUILT (121)

116 gave resistance four sources - a species, a class feature, a spell,
an item - and not the one that matters most at a table. The commonest
resistance in 5e is not on a player at all: it is "bludgeoning, piercing
and slashing from nonmagical attacks" on half the Monster Manual, and
`npcs` had no column to say it with.

TWO COLUMNS, AND THE SECOND ONE IS THE INTERESTING ONE.
`npcs.grants` is what a statblock states, in 100's vocabulary like
everything else. `characters.npc_key` is which statblock a creature was
made from - information `instantiate_npc` had been throwing away since
022. It read a statblock, copied six numbers out of it and forgot where
they came from, so nothing downstream could ever ask the statblock
another question.

A KEY AND NOT A COPY, which is `species_key` since 056 doing exactly
this. `load_npc_grants` reads the statblock when the sheet loads, so
correcting a statblock corrects every goblin already on the board, with
no backfill and nothing touched mid-fight.

THE ALTERNATIVE WAS `characters.grants`, copied at instantiation, and it
was rejected twice over. It is a snapshot, so fixing a typo leaves every
creature already on the board wrong. And a general grants column on a
character would be read by the resistance path and by nothing else -
seven `worn_grants` call sites would have to be widened to make an `ac`
target work there, and until they were, writing one would be a silent
no-op. That is the defect this codebase is named for.

VERIFIED, AND THE SECOND USER IS WHY IT MATTERED. The migration lands on
a database jec is also using, so `instantiate_npc` was exercised after
the change rather than assumed: a rolled-back probe built a creature
with `npc_key=goblin` and two kit items, and the backfill linked the one
NPC already on the board. 089's lesson was followed in the order of
statements - columns before the function that names them, because
plpgsql resolves field names at run time and a function naming a missing
column compiles clean and fails on the button.

NOT SEEDED, DELIBERATELY. The catalogue holds three statblocks and all
three are goblins, which resist nothing in 5e. Inventing a monster to
demonstrate a column would be putting game content in a schema
migration. One statement gives a statblock a resistance:

```sql
update npcs set grants = '[{"target":"resist.poison","source":"Goblin"}]'
 where key = 'goblin';
```

WHAT IS AND IS NOT EXERCISED. The schema, the backfill, the rebuilt
`instantiate_npc` and the exact query shape the loader sends are all
verified against the live database. `resist::from_grants` and
`grants::parse` are tested in Rust on this shape of input. The thirty
lines of glue between them mirror `load_species` and compile, and will
not have run against real data until a statblock carries a grant.

## What a thing is - BUILT (122, creature.rs)

Step one of the Creatures tab, and useful on its own.

FIFTEEN SPELLS ALREADY NAME A CREATURE TYPE and not one of them could be
checked, because nothing in this schema recorded what anything IS. Hold
Person works only on a humanoid; Cure Wounds and Spare the Dying do
nothing for a construct or the undead; Protection from Evil and Good,
Detect Evil and Good and Dispel Evil and Good all name the same six
types; Gentle Repose stops a corpse becoming undead.

THE SPECIES NORMALLY ANSWERS. Every Ny'ook is the same thing, so the
type belongs to the PEOPLE and is set once rather than on every
character made from them. `characters.creature_type` is the exception
rather than the rule - the one cursed Unt'garoth who is now undead - and
a statblock states its own because a monster has no species row to ask.
`creature::of` owns that order, which is the arrangement `carry_size`
already uses.

NOTHING WAS BACKFILLED TO humanoid. Every creature in the game had no
type at all, and defaulting them would assert something about this world
that its designer has not said - whether an Unt'garoth at seven to ten
feet is `humanoid` or `giant` is a design decision, not a migration's.
NULL reads as "nobody has said", which is true. The three goblin
statblocks ARE set, because a goblin is a humanoid in the book and that
is not an opinion.

**The four species are waiting on Dave:** Unt'garoth, Unt'gar, Ny'ook
and Fjell'gar all have `creature_type` NULL, and until they are set the
sheet says nothing and the spell rules that need a type will say they
cannot tell.

THE FOURTEEN ARE IN TWO PLACES ON PURPOSE - `creature.rs` for the code
and a CHECK constraint for everything that never went through it, which
is the arrangement `character_slots` has had since 107. A rolled-back
probe confirmed the constraint refuses a fifteenth type.

SIX THINGS IN creature.rs CARRY `#[allow(dead_code)]` with the spells
that will call them named in the reason - the six "evil and good" types
and the two that cannot be healed. They are tested 5e facts waiting on
the spell wiring, and the alternative was deriving them again later.

## A creature template is a character - BUILT (123, 124, commands/creatures.rs)

Stage 1 of the Creatures work. 022 settled the INSTANCE - every actor in
an encounter is a `characters` row - and left the TEMPLATE as a second,
thinner schema: `npcs` is 23 flat columns against `characters` 44 and
eight satellite tables, sharing only ten column names with the thing it
makes. Abilities are columns there and rows here. There was nowhere on a
statblock to put a skill proficiency, a prepared spell, a feature with
uses or a multiclass level, so a creature gained the capacity for all of
them the instant it was instantiated and arrived with none.

`characters.is_template` CLOSES THAT. Placing a creature is a
character-to-character copy, so everything built for a player character
works on a creature the day this lands - the ability rows, the save
buttons, the resistance block, the equipment ladder, prayers and slots.
"Edit" in the Creatures tab opens the character sheet. That is the whole
payoff and it cost no new UI.

`npcs` BECOMES A PUBLISHED REFERENCE and is otherwise untouched - the
Monster Manual, which you copy out of. A global statblock is writable by
nobody through the app, so importing is the only way to change a goblin
and the shared one stays as it was.

WHAT A COPY CARRIES: scores, skill proficiencies, class levels, choices,
prepared spells, and the whole kit INCLUDING what is inside its
containers. WHAT IT DOES NOT: spent slots, spent uses, hit point events,
death saves. A creature arrives rested and whole, and a template poked
at in the tab does not bleed onto creatures already made from it.

124 IS 123'S BUG, FIXED FORWARD. 123 copied the kit in one statement,
inventing a uuid for each container's `entity_id` so it could re-point
children before any row existed - and it fails on the first creature
carrying a container, because `entity_id` is a foreign key into
`entities` and 030 hands it out from a BEFORE INSERT trigger that only
fires for a container. The copy goes a level at a time now, taking the
id the trigger returns and carrying it down. Corrected in a new
migration rather than by editing an applied one, which is 115's rule.

A TEMPLATE IS NOT A CREATURE IN THE WORLD, and this is enforced in six
places. Five game-wide reads exclude templates - the player list,
creatures at a location, the perform audience, who rests, and holder
names - and a TRIGGER refuses to put one in an encounter. The trigger is
in the database rather than in Rust because a template in a fight is not
a display mistake: it would roll initiative, take damage and die, and
`encounter_actors` can be written by anything holding the publishable
key.

VERIFIED AGAINST THE LIVE DATABASE, all rolled back: a template copied
with 6 abilities, 1 skill, 1 prepared spell, 3 top-level items and a
torch nested inside a backpack, with slots and uses at 0 and the
template's own kit untouched; the enrolment trigger refusing with its
own message; and all five sweeps returning 0 for a template sitting in a
location. In the stub rig the panel paints, "edit" hands the creature to
the character sheet with its id, and "place" is double-click guarded.

NEW FILE BECAUSE dm.rs IS 1037 LINES, past the ceiling the architecture
note sets. `commands/creatures.rs` is 199.

STILL TO COME: Stage 3, export and import for backup and sharing. The
one decision outstanding is what an import does when a creature
references an `item_key` or `spell_key` the receiving game does not have
- skip with a warning, or refuse the file.

## A creature as a file - BUILT (125, creature_io.rs)

Stage 3, and the reason 123 could decide templates belong to a game: a
file is how one travels, to a backup or to somebody else's table.

KEYS TRAVEL, IDS DO NOT. Nothing in the file is a uuid - a creature is a
name, some numbers and a pile of keys into catalogues. That is what
makes it portable and it is also the whole problem, because the game
receiving it may not have them. A test asserts no `id`, `game_id`,
`owner_uid` or `entity_id` ever reaches the text.

SKIP WITH A WARNING, NOT REFUSE, which was Dave's call and is the right
one: a creature carrying one unknown trinket is still worth having, and
a file that will not open because of a torch is a worse answer than a
goblin with no torch. Every skip comes back by name and says what it
COSTS - losing a torch is a torch, losing a SPECIES costs ability
bonuses, a size and maybe a resistance, and the text says so.

A CONTAINER THAT GOES TAKES ITS CONTENTS. Keeping the torch when the
backpack was skipped would put a loose torch in somebody's hands, which
is not what the file said; the warning says how many went with it.

THE ENVELOPE IS THE ONE THING THAT IS REFUSED. A file that is not this
format, or is a version this build does not know, has nothing worth
salvaging and guessing would import nonsense silently. An OLDER file
stays readable - every field added since version 1 is optional, which
is what the `serde(default)`s are for.

WHAT TRAVELS: scores, skill proficiencies, class levels, the choices
made for them, prepared spells, and the kit including container
contents. WHAT DOES NOT: spent slots, spent uses, damage, death saves -
the state of one afternoon rather than of the creature, which is the
same line `instantiate_character` draws.

A BLOB AND AN ANCHOR for the download, rather than a Tauri file dialog -
that would have been a dependency and a capability entry for something
the webview already does. Import takes a file through an ordinary file
input.

VERIFIED: 17 tests on the rules, including a container taking its
contents, a choice not outliving its class, and a creature still
arriving when every single key is unknown. Against the live database, a
template with 6 abilities, 1 skill, 1 prayer and a nested kit reads out
whole. In the rig the export downloads as `cave_goblin.creature.json`
with no ids and the nesting intact, the import is double-click guarded,
and both warnings appear in the log by name with what they cost.

## A bestiary, and creatures that fight with techniques - BUILT (126, 127, 128)

123 shipped the Creatures tab with three statblocks to import from and
all three were goblins. The mechanism was finished and there was nothing
to do it with.

NATURAL ATTACKS ARE TECHNIQUES, which was Dave's call and is the better
one. 050 gave weapons named moves with their own dice, crit and fumble
ranges, a tier and a LEVEL GATE; 126 gives the same to eight limbs -
Bite, Claws, Slam, Gore, Talons, Sting, Tendrils, Hooves - three moves
each. The alternative was a flat "bite 1d6" per creature, or an item per
damage step (`bite_1d6`, `bite_1d8`), which is a catalogue of
near-duplicates with no scaling rule anywhere.

THE LEVEL GATE DOES THE SCALING, which is the whole elegance of it.
Verified on the live database: a **Wolf at level 2 reaches Snap (1d6)**
and nothing else; an **Owlbear at level 7 reaches all six** across bite
and claws - Snap, Worry the Limb, Crush the Throat, Rake, Both Paws,
Open the Belly. Not one rule was written for monsters specifically; a
creature's growing teeth and a fighter's growing repertoire are the same
mechanism.

A natural weapon weighs nothing and takes no slots, because 036's
encumbrance walk reads both and would otherwise have a bear labouring
under its own teeth. They are tagged `natural` - nothing reads that yet,
and it is where a shop will filter from.

36 CREATURES ACROSS ALL FOURTEEN TYPES, deliberately rather than tidily:
122 put `creature_type` in because fifteen spells are written against
it, and a reference where everything is a humanoid exercises none of
them. Fourteen of the 36 carry resistances, so 116 and 121 finally have
something to sit on - a Skeleton vulnerable to bludgeoning and immune to
poison, an Ice Mephit vulnerable to BOTH fire and bludgeoning, a Specter
resisting five energy types.

NO "NONMAGICAL" RESISTANCES, which is a gap rather than a choice. The
grant vocabulary cannot say "from nonmagical attacks", so the Wight and
Specter carry only their unconditional ones. Overstating a monster is
worse than understating it.

### 128 is a bug the bestiary found

`instantiate_npc` has mis-slotted kit since 022 and it never showed,
because there was one goblin and every assumption happened to hold for
it. With 36 creatures it showed immediately:

* **Hobgoblin, Wight and Bugbear arrived with their weapon in NO SLOT.**
  084 makes a null slot mean not equipped, so those three could not
  attack at all - the weapon was not in the loadout for
  `attack::resolve` to find.
* **Bandit drew its crossbow and left the scimitar on its hip**, because
  the rank was alphabetical over every kit row including carried ones.
* **Skeleton put its sword on its hip and had both hands empty**,
  because a slung bow took rank 1 from it.

Two faults: the rank counted things that are not held, and a MEDIUM
weapon that was not rank 1 fell past every branch into nothing. Now the
rank covers only equipped weapons, and a second held weapon takes the
off hand when no shield wants it. Verified by instantiating all 39
statblocks: **36 of 39 arrive with a weapon in hand**, and the three
that do not are the Shrieker, which has no attack by design, and the two
pre-existing goblin variants that have no kit rows at all.

## 134 creatures, and a retune that went too wide - BUILT (129, 130, 131, 132)

**134 statblocks in the global catalogue, not ~320.** Dave asked to
import all the SRD monsters. The SRD has something north of three
hundred and I can write down perhaps a third of them with numbers worth
trusting, so this is 95 more on top of 127's 39 rather than the whole
book. The rest are MISSING, not wrong, and the honest way to finish is a
machine-readable SRD file through `import_creature` - which is exactly
what the `odyssey1e.creature` envelope in `creature_io.rs` was built for.

**SPOT-CHECK BEFORE A SESSION RUNS ON THEM.** AC, HP and the six
abilities are SRD-derived from memory, not transcribed from a file. They
are right in shape and will be wrong in places. What IS verified is that
they are the rows intended: every creature and every kit row was MD5
digested locally and against the live database, and both matched - 95
creatures, 31 carrying resistance grants, 158 kit rows, 95 of 95 armed.

129 added eight more limbs - tail, beak, tusks, wing, pseudopod, fist,
constrict, spines - and 24 more moves, so 16 limbs and 48 natural
techniques. `fist` and `constrict` are not anatomy and are items anyway,
because the attack path needs them to be.

### 132 is 129's bug, and it had nothing to do with monsters

129 lowered the technique gates from 1/3/5 to 1/2/4, because **a creature
does not level up**. A wolf is a wolf; gating its repertoire on a
progression it will never walk left every level-2 creature with exactly
one move. That reasoning holds. The statement did not:

    update techniques set min_level = 2 where game_id is null and min_level = 3;

`where game_id is null` is not "where this is a natural weapon" - it is
"where this belongs to the global catalogue", and 043's 159 PLAYER
weapon techniques are global too. **So 129 retuned the whole game.**
Every character got Split the Collar at 4 instead of 5 and Beard the
Shield at 2 instead of 3: a balance change to player progression,
arriving invisibly as a side effect of a bestiary migration.

Caught by asking the database a question I did not need to ask - the
distinct gate values across ALL techniques - and finding no 3s and no 5s
left anywhere. 132 reverses it exactly, which is possible because
**nothing was ever authored at gate 2 or 4**: all 183 technique rows in
`supabase/migrations/` were written at 1, 3 or 5, so a weapon technique
at 2 today can only have been a 3. Now 62/62/63 weapon techniques at
1/3/5, natural ones at 1/2/4, scoped by `content_tags @> natural` - 126
put that tag in as a hook and this is the hook being used.

**The lesson is narrower than "be careful".** `game_id is null` reads
like a scope and is not one. It is the tenancy predicate, and it appears
in nearly every catalogue statement in this schema, so it is the obvious
thing to reach for when you actually mean "the rows I just inserted".
When a data migration UPDATES rather than INSERTS, say what kind of row
you mean.

### `level` on a creature is an encounter weight

127 set this scale by hand - wolf 2, owlbear 7, hill giant 10 - and it is
roughly "the party level this is a fair fight for". It is NOT a CR and
will not match one. It is also load-bearing, because it gates techniques,
so a level typed carelessly is a creature with fewer attacks than
intended. Fifteen of the 95 were corrected against the 127 scale before
the migration was written: a Giant Constrictor Snake had arrived at
level 13 next to the Fire Giant. `prof_bonus` follows 127's rule and not
the player table - 2 up to level 9, 3 above - so one catalogue has one
scale.

**39 of 134 creatures still have fewer than three live moves**, all at
levels 1-3, straight from the 1/2/4 gate: a level-1 creature reaches one
move per limb and a level-3 one reaches two. Dave asked for three
minimum. The gate and that request are in tension and I have not
resolved it, because the three ways to close it are all worse than
asking: drop the third gate to 3 (a third change to the same number in
one session), flatten the gates entirely (a giant centipede with Crush
the Throat), or invent a second limb for a rat. 92 of 134 have three or
more today. **This one is Dave's call.**

> **The count was 44, not 39, and the cause was not one thing** - found
> on 2026-10-06 by recomputing it from the database rather than
> re-reading this paragraph. Counting techniques whose `item_key`
> matches an EQUIPPED kit row with `min_level <= npcs.level`: 90 at
> three or more, 27 at exactly two, 14 at exactly one, 3 at none. Of
> the 41 that had kit, **24 were natural-weapon creatures** on the
> 1/2/4 gate as described above and **17 carried manufactured weapons
> only** - Guard, Bandit, Cultist, Orc, Skeleton - which 132 correctly
> put BACK to 1/3/5, so for those the gate above was never the reason.
> The three with nothing were two 006 orphans and the Shrieker, which
> has no attack in the SRD at all. 133 and 136 answer all of it; the
> paragraph above is left as it was written, per 115.

## A creature is not gated by a ladder it cannot climb - BUILT (133, 134, 135, 136)

**Dave's call, taken: an NPC that uses a weapon reaches that weapon's
techniques.** The fix is not where 129 looked for it. `min_level` is a
PROGRESSION gate - it says when a fighter has earned Split the Collar,
and the whole of its meaning is that the character will one day be
higher than it. A creature does not level up. `npcs.level` is an
encounter weight (127's scale), so reading one against the other
measured every monster against a ladder it will never climb, and took
its own equipment away: a level-2 guard holding a spear reached the move
authored at gate 1 and neither of the other two.

**129 moved the gates; 133 stopped reading them.** The numbers were
always fine and the comparison was not - and retuning the catalogue
retuned every player character in the game by accident, which is what
132 had to reverse. `attack::gate_level(level, from_statblock)` is the
whole rule: a character's own level, or every level there is. The three
things that gate a move - `attack::resolve`, the DM's statblock buttons
and the picker on the sheet - now read one number the sheet carries,
which is the fault `skill_mods` exists to have already fixed once. The
picker was computing it in JavaScript.

**Every creature in the bestiary now reaches three or more moves.** 132
of 132, checked live, against 90 of 134 before. No gate moved and no
player progression changed.

### 134. A weapon may name its own ability

Every attack in this engine was STR or DEX, which is true of everything
you hold and false of a Shrieker's scream: STR 1 and DEX 1 gave the
fungus -5 to be heard. An item whose `properties` carry one of the six
ability codes uses that instead - specific statement beats finesse and
beats the mode - and `ability_for` takes one modifier lookup rather than
six arguments that are unused on every weapon anybody holds. Nothing in
the armoury carries one, so every existing weapon behaves exactly as it
did.

### 135. Two statblocks that were never creatures

`Goblin Fighter` (0000A1) and `Litmor` (100003) came off the 006
spreadsheet port, had no kit at all - equipped or carried - and would
have given a DM a creature that cannot act. Deleted rather than armed,
because the bestiary already has a goblin that somebody designed, and
`Litmor` was one character's name rather than a kind of thing. Nothing
referenced either: checked by query, since `characters.npc_key` is a
loose text reference and no foreign key in this schema points at `npcs`
at all.

### 136. And the Shrieker has something to do

Three shrieks off one organ, by Dave's licence to invent them: Piercing
(1d6 thunder, and a day of -5 Perception on a failed CON save),
Confusion (1d4, loses its next action on a failed WIS save) and Stunning
(1d8, stunned to the end of its next turn). It rolls on CON at +2.

**It is AIMED, and that is a choice worth knowing about.** A scream
wants to be an area with a save and this engine has no save-DC attack
path. Modelling the area anyway would mean a to-hit roll the table is
supposed to ignore, which is this codebase's named defect wearing its
usual costume, so the fungus screams AT whatever disturbed it instead -
the precedent `spines` already set.

**The riders are prose, like every rider in the game.** A grapple is
prose, a knockdown is prose, and so is the ringing in somebody's ears.
Nothing lets a technique land an effect, and the text says so outright
rather than implying the engine has it. `effects` with a `skill.prc`
grant is the shape that wants, and `apply_effect` has no UI and no
`grants` parameter - two reasons it is not built today.

## A creature is proficient with its own kit - BUILT (137)

**Dave: every creature weapon and natural attack is proficient by
default, and anything added in customization is set at that time.** The
data was nearly there - 223 of 224 global kit rows already said so,
including all 125 natural weapons - so most of this is making the
DEFAULT true rather than trusting every future seed to remember.
`npc_items.proficient_override` now defaults to true.

**The fault underneath was that NULL meant two different things.** It is
a tri-state and the nullability is the point: true or false came from
somebody, NULL means nobody has said. Two things read a NULL on a
creature's kit and they disagreed -

| | NULL reads as |
|---|---|
| `equipment::load_npc_kit` | proficient (`unwrap_or(true)`) |
| `equipment::is_proficient` | derive it from `weapon_profs` |

- so the DM's statblock preview and the instantiated creature could
answer differently about the same row. The named defect with a to-hit
attached rather than an error.

**And the derived answer is the wrong one for a monster.** Several
statblocks carry `weapon_profs = {sim}` while holding something martial
- the Goblin holds a scimitar - so deriving says NOT proficient and
takes the proficiency bonus off the creature's own weapon. It never bit
anybody because 131 set the flag on every row it wrote; the single row
it did not write is 022's goblin handaxe, the oldest kit row in the
schema, which survived only because a handaxe is simple and the
derivation happened to come out true.

**Widening `weapon_profs` would have been the wrong fix** and is worth
saying out loud: giving the Goblin `mar` makes the next martial weapon
anybody hands it proficient too, which is the opposite of what was
asked. Proficiency with what a statblock SHIPS is a different fact from
proficiency with a CLASS of weapon. So it is stated per row, the default
states it, and `instantiate_npc` resolves a NULL to true on the way to
the object rather than passing it on to be derived.

Live after: 224 of 224 kit rows proficient, no creature object left
deriving, and still exactly one `instantiate_npc` - 095's overload trap
avoided by keeping 124's signature byte-for-byte.

**One thing deliberately left**: `load_npc_kit`'s `unwrap_or(true)` and
`is_proficient`'s derivation still differ in the source. No data reaches
them as NULL any more, so nothing can see it, but the two answers are
still written in two places and the comment saying why they now agree
was not added - the app was mid-test and touching `src-tauri` restarts
it, which signs the session out. Worth a line when next in there.

## A creature you made could not be found - FIXED (138)

**Dave made seven creatures on the Creatures tab and could not enrol
one.** They were all there - seven templates in Test Game 1, written
correctly by `import_statblock` - and nothing in the encounter panel
could see them.

The enrol picker offered two lists: `list_npcs`, the shared bestiary,
and `list_characters`. **`list_characters` is players only**, and
correctly so - since 022 a monster is a character too, and without that
filter a player's list fills with goblins. But it is the only list the
picker had for "somebody who already exists", so **every NPC in the game
was unreachable**: a template from the Creatures tab, a creature placed
into the world, Pete left standing after the last fight. A DM could make
a creature and then have nowhere to put it.

**`list_individuals` is the missing list** - every non-template character
in the game, players and creatures both, with `is_npc` travelling on the
row rather than split into two queries. It is a label and not a
structure, so the screen groups on it and nothing else has to care.

**And `enrol_actor` now takes a template.** Three kinds in, one kind out:
a reference statblock becomes an individual, a template becomes an
individual through `instantiate_character` - the same RPC "place in
world" already calls - and somebody already standing is enrolled where
they stand. Saying it here keeps 123's rule, that a template is never
itself in the world, in one place rather than leaving the screen to make
two calls in the right order. A copied template rolls its own initiative
on the way in, like any other monster; it had been dropping to the
bottom of the order with a button beside it.

**Four groups in the picker, and they say what will happen:**

| group | on enrolment |
|---|---|
| This game's creatures | a copy is placed |
| In the world | enrolled as they stand |
| Player characters | enrolled as they stand |
| Reference statblocks | a copy is placed |

This game's own creatures sort first, because 132 reference statblocks
above them would bury seven templates - which is most of how the
original bug felt even before it was one.

**The picker refreshes when the Creatures tab changes.** A list that
caught up only when the DM pane was next built would be the same bug
with a shorter fuse.

## What the enrolment and the fight actually did - EXAMINED, one FIX (139)

**Read off the live rows after Dave tested 133 to 138.** The Tavern, Test
Game 1, round 3: Falon (init 6), Luci (4) and Webbys the giant spider
(18, and it is Webbys' turn).

**Everything built today is confirmed working, from the data rather than
from the screen:**

| | |
|---|---|
| 138 | Webbys the TEMPLATE was enrolled at 19:59 and a copy was made - scores, HP 26, AC 14, size lg, beast, bite in the right hand, all identical to the template, and `location_id` set to the Tavern |
| 138 | the copy rolled its own initiative on the way in: 18 |
| **133** | at 20:03 it used **Crush the Throat**, which is `min_level = 5`, and **Webbys is level 4**. Before 133 that technique did not exist for any creature in the bestiary |
| 137 | to hit +4 = STR 14 (+2) + prof 2, so it was proficient with its own bite |
| 054/060 | the action is stamped round 3, `turn_actor_id` = Webbys, so it was ON its turn and not out of turn, `cost = attack` |

The roll itself: natural 19, crit (the technique's own `crit_min` is 19,
`fumble_max` 2), 23 against Falon's AC 14, then 4d6+2 for 17 - the crit
doubling 2d6 into 4d6.

**`reason: "auto_hit"` on a natural 19 is intended, not a fault.** A
widened crit auto-hits the same way a 20 does, and
`resolution.rs::a_widened_crit_auto_hits_the_same_way` is the test that
says so. Checked because it looked wrong.

### 139 is 138's bug, and it is about names

**138 gave the actor no provenance, and 018's naming turns out to depend
on it.** `name_actor()` had two branches: `npc_key` set means "Goblin
0001", numbered across the game; `npc_key` null means take the
character's name verbatim. A player enrolled as themselves wants the
second - Falon is Falon. 138's copies landed there too, because the
statblock key is on the CHARACTER row and not on the actor.

So the first Webbys is "Webbys" and **so is the second, and so is the
third** - three actors with one name, indistinguishable in the roster,
in the turn order, and in `rolls.character_name`, which snapshots the
name at the moment of the roll and cannot be untangled afterwards. The
many-goblins problem 018 exists to solve, reached by a path 018 could
not see.

**Stamping the statblock key on the actor would have been the wrong
fix** and is worth saying: Webbys' template carries `npc_key =
giant_spider`, so the existing branch would have numbered it off the
SPECIES and renamed the character to "Giant Spider 0001", deleting a
name the DM chose on purpose. The point of a template is that it is
yours and it is named.

So `encounter_actors.template_id` records which template a copy came
from, and `name_actor` gains a third branch that numbers off the
TEMPLATE'S name: Webbys 0001, Webbys 0002. The ordinal block is shared
with the statblock branch rather than written twice.

**Webbys as it stands was left alone, both its name and its
provenance.** Renaming mid-fight is obviously wrong - two resolved roll
rows already carry `character_name = 'Webbys'`, and 018's argument is
that a name already rolled under is a fact. Backfilling the id was
subtler: a character names the STATBLOCK it descends from, not the
template, so the only route back is matching `npc_key` against this
game's templates - and **there are two Goblin templates**. The match is
not unique, so the join would have let Postgres choose. A guess written
into a provenance column reads exactly like knowledge, which is the
fault this codebase is named after; a null at least says "came in
before 139".

## A creature is a character and is not a person - BUILT (140)

**A third sub-tab on Characters: PCs | NPCs | Creatures.** `is_npc` was
the only split the roster had, and it answers a different question - a
merchant somebody wrote and a goblin stamped from a statblock are both
NPCs, and the second kind arrives in tens.

**The line is 5e's type, and humanoid is the one that is people.** Of
the fourteen, that is the whole of it: a hill giant, a wolf and a
skeleton are all things you fight, and an elf shopkeeper is not. Dave
chose this over "did it come from a statblock", which would have filed a
merchant built on the commoner statblock as a creature.

**Unstated stays among the people**, which follows 122 rather than being
a new decision. A type nobody has filled in is unstated, not secretly
anything. A monster on the wrong tab for want of one word is visible and
fixable; a person quietly filed as a monster because a column was empty
is not.

**`creature::is_creature` owns the rule and `who_is_where` carries the
verdict.** It is one word compared against a constant, which is exactly
the kind of thing that gets written in two places and then disagrees -
`skill_mods` is the scar, and the roster is a screen that has no
business deciding what a thing is. The command now maps its rows and
adds `creature`; the screen reads it.

Each row carries what it IS and what it came FROM - the type and the
statblock key - so a creature says why it is on this tab.

**A named creature is still a creature**, which was Dave's own question:
naming Webbys does not make it a person, and all three lists are the
Characters tab either way, because since 022 a creature IS a character.

## Four bestiaries in one dropdown - FIXED (141)

**138's own bug, and it is a shape rather than a typo.**
`loadStatblockPicker` cleared the select and then awaited three calls
before appending anything. Two overlapping runs therefore both cleared
an empty list and both filled it, and the enrol dropdown held the
bestiary three or four times over.

**`innerHTML = ""` at the top of an async builder looks like it makes
the function idempotent and does the opposite.** It moves the clearing
to a moment that has nothing to do with the appending, and everything
in between is somebody else's turn.

Nothing overlapped until 138, which made it ordinary: the picker is now
called from the Creatures tab as well as from the DM pane, so a tab
switch and a refresh can be in flight together.

**The fix is a run counter and a fragment.** The late run drops its work
instead of adding it; the list is built off-screen and swapped in one
go, so the select is never momentarily empty; the three calls go in
parallel rather than one after another; and the selection survives the
rebuild when what was picked is still on offer. That last one is the
fault the note at the top of `main.js` already describes for the target
list - refreshing a list under somebody is how you make them pick the
goblin twice.

### The same shape is in eighteen other builders

Found by sweeping for it rather than by guessing, and **not fixed**,
because each one is a real change and none is currently broken. They are
only safe because they are not called concurrently today, which is
exactly what was true of the picker until 138. Ranked by how many places
call them - the number is not a bug count, it is how much opportunity
there is:

| call sites | |
|---|---|
| 19 | `loadSheet` |
| 16 | `selectEncounter` |
| 15 | `loadDM` |
| 9 | `loadRolls` |
| 8 | `loadWorld` |
| 6 | `loadObjects`, `loadCharacters` |
| 4 | `loadCreatures` |

**The cheap fix is one helper**, not nineteen edits: a `paintInto(sel,
builder)` that owns the run counter and the fragment swap, applied as
each is next touched. Worth doing before the next screen is built on
the same shape.

## Seventy-one more of the SRD - BUILT (142)

**203 statblocks in the shared bestiary, up from 132.** Dave asked for
more creatures, and the holes were specific rather than alphabetical: no
dragon older than a wyrmling, no lich, no vampire, no golem, no
lycanthrope, and none of the beasts a table actually asks for - a
mastiff, a riding horse, an elephant.

| | |
|---|---|
| Dragons | young and adult of all five chromatics |
| The classics | lich, vampire, vampire spawn, ghost, banshee, medusa, manticore, chimera, hydra, treant, unicorn, roc, mimic, oni |
| Fiends | balor, marilith, vrock, hezrou, glabrezu, nalfeshnee, lemure, bone devil, erinyes, horned devil, pit fiend |
| Constructs, giants | the four golems, shield guardian, cloud and storm giant |
| People | commoner, noble, bandit captain, assassin, gladiator, drow, duergar, sahuagin, yuan-ti pureblood |
| Lycanthropes | all five |
| Beasts | fifteen, from a mastiff to a giant ape |

**Verified, and the verification found nothing - which is the point of
doing it.** Every item key checked to exist BEFORE applying (131's
failure was a kit row naming an item that did not); `prof_bonus` checked
against 127's rule on all 71; every grant parsed as JSON; and after
applying, all 71 rows diffed field by field against the file. **All 203
creatures reach three or more moves and none is unable to act.**

The first digest comparison came back MISMATCHED and that was the
checker, not the data - Postgres `order by` and Python `sorted()`
disagree about a hyphen. Worth remembering before trusting a digest: a
sorted join compares the sort as well as the content.

**Still SRD-from-memory, and the famous ones are better than the obscure
ones.** A pit fiend and a werewolf are numbers that get read often; a
nalfeshnee is not. Spot-check anything before a session runs on it. This
is the last batch worth writing by hand - `creature_io.rs` reads an
`odyssey1e.creature` envelope and a machine-readable SRD dump through it
beats more recollection.

### "From nonmagical attacks" is not expressible, and 23 of these want it

The commonest resistance in 5e is "bludgeoning, piercing and slashing
FROM NONMAGICAL ATTACKS" - every lycanthrope, both vampires, all four
golems, most fiends. 116's vocabulary has `resist.bludgeoning` and no
way to say the qualifier.

**Recorded as plain resistance, following 131** - the Intellect Devourer
has carried it that way for a month, and a bestiary where one rule is
written two ways is worse than one where it is written imprecisely. But
the cost is real and the direction is backwards: **a party's magic sword
is halved against a werewolf**, which is the opposite of what the rule
exists to do. The qualifier IS the rule and we are dropping it.

The fix is a target that carries it - `resist.slashing.nonmagical` -
read by `resist.rs` and checked against the weapon's own grants, since
100 already knows whether a weapon is enchanted. Not built with 142
because it changes how 131's creatures resolve too, which is its own
change with its own verification.

### 142 cannot be recovered from the database

Its `schema_migrations.statements` carries the applied SQL with the
header replaced by one line pointing at the file. The SQL is identical;
the reasoning is not. Every other migration here can be recovered
byte-for-byte - this one has git as its only full record.

## "From nonmagical attacks" - BUILT (143, resist.rs, attack::Attack::magical)

**The qualifier is the rule, and it was the half we were dropping.** The
commonest resistance in 5e is "bludgeoning, piercing and slashing FROM
NONMAGICAL ATTACKS" - every lycanthrope, both vampires, all four golems,
most fiends, the lich, the ghost. 116 gave this schema
`resist.bludgeoning` and no way to say the second half, so 131 and 142
recorded the resistance and lost the condition. A party that finally
found a magic sword watched a werewolf halve it exactly like the stick
they started with: the named defect arriving in the one place a player
was supposed to feel rewarded.

**A suffix, not a fourth degree.** `resist.slashing.nonmagical`, and the
three-type form half the Monster Manual wants in one target:
`resist.bludgeoning|piercing|slashing.nonmagical`. "Resistant" and
"resistant to nonmagical" are the same degree under a condition, so
`Degree` stays the three 5e has and everything reasoning about halving
and doubling is untouched.

**A misspelt qualifier is refused, not ignored** - `resist.fire.nonmagicl`
would otherwise read as a working target and apply in every case its
author meant to exclude, which is the same argument the unknown-damage-type
refusal has been making since 116.

**What counts as magical is the weapon's own grants.** 100 made an
enchantment a list of grants, so a weapon carrying one is enchanted.
`attack::Attack::magical` reads `owned.grants` and NOT everything
`reaching` gathers - a ring granting +1 to attack does not make the
sword magical, it makes the swing better, and that distinction is
exactly the question the qualifier asks.

**Gap left, stated rather than hidden**: a weapon that is magical and
grants nothing has nowhere to say so. It wants a flag of its own on the
item, and nothing in the catalogue needs one yet.

### The standing is settled twice, and a test caught why

`resist::standing` now resolves each damage type over two sets of
sources - all of them for an ordinary attack, and only the unqualified
ones for a magical attack. Filtering after resolving would get the mixed
case wrong: a creature immune to fire from ordinary weapons and merely
resistant to it in general is RESISTANT to a flaming sword, not immune
and not untouched.

**The first version dropped the whole standing when the ordinary case
cancelled**, which lost the magical answer with it - a werewolf under a
curse that doubles slashing resists an ordinary blade and should take
DOUBLE from a magic one, and was taking neither. Found by a test written
for the case rather than by the compiler.

**Nothing is said about a resistance that did not fire.** "resistant
slashing - 9 instead of 9" reads as a rule that worked; silence is the
honest answer.

### 21 creatures qualified, and three deliberately not

Only the ones whose SRD text carries it: the five undead, six fiends,
four golems, five lycanthropes, and 131's Intellect Devourer, which had
carried the unqualified form longest. 63 targets rewritten, order and
every other grant preserved.

**Left flat on purpose** - the Treant and Awakened Tree (a magic axe is
no better against wood), the Swarm of Insects (there is nothing to hit
either way), the two oozes' slashing immunity, and the skeletons'
bludgeoning VULNERABILITY, which the qualifier has nothing to do with.

**Not fixed in 143, done in 144**: the Wraith, Specter and Shadow should
have this resistance and did not have it recorded at all - 131 wrote
them with elemental resistances and no physical ones. Adding a
resistance is a different act from qualifying one that is already there,
so it got its own migration.

**Also still not expressible**: "that aren't adamantine" on the golems
and "that aren't silvered" on the devils and lycanthropes. Both are
narrower than this one and both make the resistance apply MORE often, so
what is recorded now is a move toward the printed rule rather than away
from it.

## The ones that never had it recorded at all - BUILT (144)

**Dave asked for the Wraith, the Specter and the Shadow. It was
seventeen, and four of them were mine.** 143 fixed the creatures whose
physical resistance was written down without its condition; this fixes
the ones that never had it written down at all.

142 gave the Vrock, Hezrou, Glabrezu and Nalfeshnee their elemental
resistances and stopped - every demon in the SRD also resists
bludgeoning, piercing and slashing from nonmagical attacks, and I wrote
the first half of each of those lines the day before. Fixing the three
asked for and leaving four I had just broken would have been a strange
place to stop.

| | |
|---|---|
| Wraith, Specter, Shadow | the three asked for - incorporeal undead |
| Vrock, Hezrou, Glabrezu, Nalfeshnee, Dretch | 142's demons, written with their elements and not their hides |
| Mummy, Wight | 131's undead |
| Gargoyle, Helmed Horror | 131's constructs |
| Imp, Quasit, Succubus | 131's lesser fiends |
| Deva, Grick | the celestial and the one monstrosity with it |

**38 creatures now carry the qualified physical resistance**, up from
21. No duplicates, every existing grant preserved - the Shadow kept all
eight of its own and gained three.

**Deliberately not given it**, each for a reason rather than for want of
checking: Ghast and Ghoul (undead that can simply be hit), Animated
Armor, Flying Sword and Scarecrow (poison and psychic immunity and
nothing else), and the Lemure, which is exactly as stabbable as it
looks.

**One fact per migration.** This adds the physical resistance and
nothing else, even where something else is also missing - **the Specter
should be IMMUNE to necrotic and is not**. That is a different fact with
a different way of being wrong, and bundling it would mean a migration
whose name stops describing what it did. Worth a sweep of its own: the
bestiary was written in batches from memory and this is the second
omission of the same shape found by looking.

## All 203, against the book - SWEPT (145)

**Twice in two days a hole turned up by looking rather than by playing**
- 144 found seventeen where Dave had asked about three - which is the
signal to stop patching and read the whole thing. Every creature in the
shared bestiary, its recorded resistances against what the SRD prints.
**203 checked, eleven wrong.**

**Four of the five elementals** resist bludgeoning, piercing and slashing
from nonmagical attacks and **not one of them said so**. The Earth
Elemental is also vulnerable to thunder, which it did not say either.
The Gargoyle - an elemental too - was given its line in 144, and that is
how the family came to be looked at.

| missing | |
|---|---|
| Air, Fire, Water Elemental | b/p/s from nonmagical |
| Earth Elemental | the same, **and** vulnerable to thunder |
| Swarm of Rats | resists b/p/s **flatly** - a swarm is not hard to hit, there is simply too much of it. Swarm of Insects has had it since 131 and the rats were written beside them without it |
| Shield Guardian | immune to poison |
| Dust Mephit | vulnerable to fire - the other three mephits all carry theirs |
| Ghast | resistant to necrotic |
| Specter | immune to necrotic - the gap 144 found and deliberately left |

| wrong degree | |
|---|---|
| Wraith, Shadow | necrotic is **immunity**, not resistance |

That last pair is the subtler kind and not a rounding difference: half
of a large necrotic hit still kills somebody and none of it never does.
It is the difference between a Wraith the party's necromancer can wear
down and one they cannot touch at all.

**What was checked and is right** - because "I looked and it was fine"
is a result: every dragon's element, all four golems, the three
remaining mephits, both other oozes, the plants (the Treant and Awakened
Tree resist bludgeoning and piercing FLATLY, since a magic axe is no
better against wood), the Banshee and the Ghost, whose long lines are
exactly right, the Lich, both vampires, the Wight, the Mummy, every
lycanthrope, all eleven fiends, and the forty-odd beasts and humanoids
that correctly have nothing.

**After: 329 grant targets across 90 creatures, 126 of them qualified.**
Every target checked against the grammar `resist.rs` accepts, so none is
silently inert; no duplicates; none without a source to name on a sheet.

### What a sweep like this is worth, and what it is not

**It compares the database against my recollection of the SRD - the same
source that wrote the rows.** It catches what 131 and 142 FORGOT, and the
evidence says that is the common failure: every one of the eleven is an
omission or a degree, not an invention. It cannot catch what I have
remembered wrongly the same way twice.

**A second reader is worth more than a third pass by me.** The honest
version is an SRD file through `import_creature` - the argument 142's
header already made. Until then these numbers are right in shape and
worth spot-checking.

**One left alone for want of confidence**: the Gas Spore. I do not trust
my memory of its line, and a guess there would be indistinguishable from
the eleven above.

## The bestiary is the book now - BUILT (146, 147)

**145's header asked for a second reader. Dave found one: the SRD is
published online.** The whole bestiary was read against it - every
armour class, hit point total, ability score and resistance, with the
resistances parsed into 116's vocabulary mechanically rather than
compared by eye.

### 146: fifteen wrong out of 184 matched

**Immune, not resistant - ten of them.** All five lycanthropes, all four
golems and the Lich are IMMUNE to bludgeoning, piercing and slashing
from nonmagical attacks. 143 built the qualifier and 144 spread it, both
reading "resistance" off a memory that had the condition right and the
degree wrong. **A werewolf was taking half from an ordinary sword and
should have been taking none** - a bigger error than the one 143 existed
to fix, sitting underneath it the whole time and invisible to three
passes that were all checking WHETHER the line was there.

**And one 144 got backwards.** The Dretch has no physical resistance at
all. 144 gave it one on the reasoning that "every demon in the SRD also
resists b/p/s from nonmagical attacks" - true of the vrock, hezrou,
glabrezu and nalfeshnee, false of the dretch. **The error was not a bad
memory, it was confidence in a pattern**, which is the shape worth
remembering.

The rest: Horned Devil 148 to 178 hit points, Werewolf AC 12 to 11,
Wereboar 12 to 10, Giant Scorpion DEX 13 to 11, Wight immune to poison,
Assassin resistant to poison. **169 of 184 matched exactly on every
field** - the numbers written from memory were, on the whole, right, and
the failures cluster in degrees and in families rather than in digits.

### 147: the 77 the SRD has and we did not

**280 creatures.** The dragons are finally complete - we had five
chromatic colours at three ages, the SRD has ten colours at four, so the
metallics and every ancient dragon were missing. **43 dragons now.** The
top end did not exist at all: the bestiary stopped at the Pit Fiend and
now runs to the Tarrasque by way of the Kraken, the Solar, both sphinxes
and the Purple Worm. Plus the dinosaurs, hags, nagas, genies, four more
devils, and the ordinary-looking things a dungeon needs.

**These numbers are READ, not recalled**, which is new. 142's warning -
"SRD-derived from memory, right in shape and wrong in places" - does not
apply to this batch. The prose is ours: every `notes` line is written
for this bestiary.

**One deliberate deviation, and it is a balance decision.** 127 set
`prof_bonus` by its own two-step rule, 2 up to level 9 and 3 above, "so
one catalogue has one scale". That was written when the catalogue topped
out near CR 10, and following it to CR 30 would give the Tarrasque +3 -
it would hit less often than a guard captain. So these 77 use 5e's own
proficiency by CR, which the column has always allowed.

**The existing 203 are NOT retuned, and that restraint is 129's lesson
paid forward**: a balance change arriving as a side effect of a data
migration is exactly what 132 had to reverse. The catalogue now has two
proficiency scales and this is where that is written down. Deriving the
old ones from CR is a one-line change whenever Dave wants it, and it is
a change to how 203 existing creatures hit.

### Verified after

280 statblocks, 554 kit rows, every one proficient (137), **all 280
reach three or more moves and none is unable to act** (133, 135), no
grant target the engine cannot read, no kit row naming an item that is
not there, top level 30.

### What is still missing, and it is not numbers

**Breath weapons, legendary actions, lair actions, regeneration and
spellcasting.** A dragon here bites, claws, lashes and buffets; it does
not breathe. That is a whole mechanism rather than a column, and
inventing half of one for 43 dragons would be worse than the honest
absence - the statblock screen shows the prose and the breath stays a DM
call, as it has since 127.

**Nineteen creatures are ours now, by discovery rather than design.** Six
we invented - Goblin Boss, Guard Captain, Orc War Chief, Harpy Matriarch,
Dire Boar, Kobold Dragonshield. The other thirteen have no page on that
SRD either: Archer, Banshee, Pixie, Acolyte, Druid, Carrion Crawler,
Hook Horror, Helmed Horror, Intellect Devourer, Scarecrow, Twig Blight,
Yuan-ti Pureblood and the Gas Spore. They were written from memory of a
book they are not in, and nothing has verified them.

## A dragon that can breathe - BUILT (148, attack::damage_mod)

147's header listed what the bestiary still could not say and breath
weapons were first: "a dragon here bites, claws, lashes and buffets; it
does not breathe." 43 dragons, and the thing everybody at the table is
actually afraid of was a sentence nobody had written down.

**Dave's read was right: it fits the existing vocabulary.** A breath
weapon is a natural weapon - an ITEM with its own dice and damage type,
equipped like a bite, rolled through the same `attack::resolve` as
everything else. No new mechanism, no new table.

**41 breath items, one per dragon,** which looks wasteful and is not.
The dice differ by colour AND age: a red wyrmling breathes 7d6 and an
ancient red 26d6. A shared `breath_fire` carrying four techniques at
four gates is the obvious shape, and **133 is exactly why it cannot
be** - a creature clears every gate, so a wyrmling equipped with it
would reach the ancient's breath. The gate stopped being available as a
discriminator, so the item has to be one. That is 133 working as
intended rather than getting in the way.

**Aimed, with the real rule in the prose** - 136's choice for the
Shrieker, for the same reason: this engine rolls to hit against an AC
and has no save-DC path. Each item's description carries the shape, the
DC and which save, and points at `encounter_challenges`, where 011's own
comment says "the roll path cannot tell them apart" from an actor's AC.
The DM adds the DC as a challenge, the table rolls saves against it, and
the item supplies the damage. Two clicks rather than one button, said
out loud rather than left to be discovered.

### `nomod`, and why a dragon's breath was wrong by +9

`properties` carries `con` - 134, the Shrieker's lesson - and `nomod`,
new here.

**5e puts an ability modifier on the damage of a weapon you SWING**,
because the arm is doing the work, and puts none on a dragon's breath.
The engine had no way to say so, so an ancient red's breath came back as
**26d6+9**: a plausible number, correctly derived from the right rule
applied to the wrong kind of thing. `attack::damage_mod` reads the
property and nothing else does; nothing a person carries has it.

Verified: 41 items, 41 dragons armed, ancient red at 26d6 fire. The only
dragons without a breath are the Pseudodragon and the Wyvern, which
correctly have none.

### The other four, and where each one would go

Dave's wider point - that these should be expressible as attacks or
techniques inside what already exists - holds for two of the four and
not for the others. Written down rather than guessed at next time:

**Legendary actions FIT, and most of the work is done.** A dragon's
legendary actions are a tail attack and a wing attack, and `tail` and
`wing` are already items in its kit. They want techniques with
`special_text` saying what they cost, and 054/060 already stamp an
action with the round and whose turn it was - `spent.rs` has
`out_of_turn` for exactly this. The gap is that nothing yet says a
creature HAS legendary actions or how many, which is a column on `npcs`
and a line on the statblock screen.

**Lair actions do NOT fit a creature at all.** They belong to the LAIR,
fire on initiative 20, and happen whether or not the creature acts. The
nearest existing home is `encounter_challenges`, which is already how a
DM authors a difficulty with a DC - a lair action is a challenge the
encounter owns rather than a technique the dragon owns.

**Regeneration is not an attack and has no hook.** It is "at the start
of its turn, regain N hit points", and nothing in this engine runs at
the start of a turn. The clock (092) and effects (094) tick on game
time, not on turn order. It wants a turn-start event, which does not
exist, and the Troll has carried it as prose since 127.

**Spellcasting has a whole subsystem already** - 101-107, `prayers.rs`,
`spellcast.rs` - and it is cleric-shaped: `character_prayers` keyed to a
character, slots from `prayers::slots_at` and a class level.
`instantiate_npc` copies kit and scores and no spells. A spellcasting
creature wants its prayers copied in the same breath as its weapons,
which is a change to one function and a new kit-shaped table rather than
a new mechanism.

## What a creature may do out of turn - BUILT (149, spent.rs)

148's header said legendary actions were the one of the four that FITS
and that most of the work was already done. It was:

| | |
|---|---|
| 054 | stamps the round onto every action |
| 060 | stamps whose turn it was |
| `spent::out_of_turn` | has compared the two since 054 |
| `spent.rs` | counts what a creature spent in a round |

Everything needed to say "the dragon has used two of its three" was
there except the three.

**A column, because there is no rule to derive it from.** Every other
budget in `spent::Budget` is worked out - attacks from Extra Attack, and
the action, bonus, reaction and free interaction are one each because 5e
says so. A legendary allowance is a number printed on a statblock.
`npcs.legendary_actions`, read through `npc_key` the way 121 reads a
monster's resistances: a fact about the TYPE, not copied onto every
individual.

**Thirty creatures, read off the published SRD, all with three** - the
twenty adult and ancient dragons, both sphinxes, the Aboleth, Kraken,
Lich, Mummy Lord, Solar, Tarrasque, Vampire and the Unicorn. Young
dragons and wyrmlings have none, which is why it is seeded by name and
not by creature type. The Dragon Turtle has none either, which surprised
me enough to check twice.

### It changed the counting, which is more than it sounds

**An out-of-turn action now has its own tally instead of spending the
creature's ordinary budget.** That is a correctness fix arriving with
the feature: an action taken on somebody ELSE'S turn does not spend this
creature's turn, and counting it there made a dragon doing exactly what
the book says - three legendary actions, then its own turn - read as
four actions and light up as over budget.

061 already had to fix a warning that fired on correct play once, for
Extra Attack, and wrote down why: a warning that fires on correct play
is worse than no warning, because a DM learns to ignore it and then
misses the one that mattered.

**Zero is the interesting value.** Almost nothing has legendary actions,
so almost any out-of-turn action is now flagged - a goblin swinging on
the wizard's turn is precisely what a DM wants to see, and it is what
054's `beyond_one_turn` was reaching for before it could tell that case
apart from Extra Attack.

**A reaction keeps its own slot.** It is the only out-of-turn thing
`cost` can name, and an opportunity attack reading as a legendary action
would be the usual costume on the usual fault.

The screen says it on hover, through `spendTitle`, which both tally
sites already call - and it says two different sentences, because "2 of
3 legendary" and "1 action out of turn, and it has none to spend" are
not the same fact.

### What it is still not

**It refuses nothing**, which is 051's decision about turn order and
061's about budgets, held to.

**It does not know a wing attack costs two.** 5e prices some legendary
options at two of the three, and no technique in this catalogue says
what it costs - 054 refused to guess that for 193 rows and the refusal
still holds. Every legendary action counts as one, and the day
`techniques` carries a cost this reads it instead.

**The reset is per round**, where 5e resets at the start of the
creature's own turn. The two differ only for an action taken between the
top of the round and that creature's initiative, and the round is the
only window `actions.round` can express.

## A creature casts the way a character does - BUILT (150, 151)

Spellcasting was the last of 147's four and the only one with a whole
subsystem already built - 101 to 107, `prayers.rs`, `spellcast.rs`, a
108-spell catalogue, slots that can be spent. **None of it could be
reached by a monster.** Six commands found a cleric class on the sheet
and refused anybody else, and a monster has no classes at all, so a
Lich - an 18th-level spellcaster in its own statblock - could not cast a
cantrip.

### The answer was not a second mechanism, and Dave stopped me building one

I was two queries from adding `npcs.caster_level` and
`npcs.casting_ability` and a creature-shaped path beside the
character-shaped one. Dave's question - shouldn't creatures work the way
PCs do - is the one this codebase has had to learn twice: 022 deleted
ninety lines of monster sheet-building and left `load_actor_sheet` at
fifteen, because a monster's sheet is loaded by the function that loads
anyone's.

**So a creature that casts has CLASS LEVELS.** `character_classes` is
what a player character has, `sheet.classes` reads it, and
`prayers::caster` finds a casting class there without caring what kind
of thing it belongs to. `npcs.class_key` has existed since 064 and was
set on exactly one statblock; what was missing was a level to go with
it, and `instantiate_npc` writing the class row rather than only copying
the key onto the character.

**One column instead of two, and it buys the player path rather than a
parallel one.** `class_level` is not `level` - 147 set `level` to the
challenge rating and the book prints both: a Priest is CR 2 and a
5th-level caster. Reusing it would have taken Spirit Guardians off a
priest.

### And the correction Dave's question forced

I had said a monster is "known, not prepared from a book" and proposed a
third shape for it. **That was wrong** - a Lich's statblock describes
*prepared wizard spells*. It is the wizard shape, and I had invented a
mechanism to describe something an existing one already covered.

### Cleric and wizard differ in the SOURCE, and everything follows

101-107 built one shape and called it prayers, and the name hid the
question. A cleric draws from the WHOLE list every day and prepares a
subset; a wizard may only prepare what is written in a book.
`prayers::Source` is the vocabulary - `whole_list` or `book` - and
`prayers::casts` is the table of which class casts on which ability from
which source. A rule rather than a column: `classes` has never said
anything about spellcasting, and this is 5e's own table.

**So `prepared` could not stay a boolean.** It was carrying two meanings
- true for prepared, false for cantrip, with every false row in the
database a level-0 spell - and a wizard needs a third value for a spell
in the book and not prepared today. One column, three states, which is
what it was always describing.

`prayers::may_reach` is where the difference lives: a cleric is checked
against `spells.classes`, and a wizard against **what they have already
written down**, because being a wizard spell is not enough.

### 151: three clerics and two wizards

Priest (cleric 5), Cult Fanatic (4), Acolyte (1), Mage (wizard 9), Lich
(wizard 18). Every list checked against `prepared_max` before seeding,
because seeding past it would fire a warning from the seed rather than
from play - Priest 8 of 8, Lich 23 of 23.

**The wizards came out better than expected.** 101-105 seeded the cleric
list, so the Mage and Lich should have been threadbare; 34 of those
spells are on the wizard list too, at every level from cantrip to 9th.
The Lich casts up to Gate and Astral Projection.

### What is deliberately absent

**The known casters** - bard, sorcerer, warlock have a fixed list and
never prepare, a third shape absent from `prayers::casts` rather than
listed and quietly treated as clerics. **The half-casters** - paladin
and ranger prepare on a half slot table, and `slots_at` is the full one.
**The wizard's acquisition path** - a player wizard's book grows two
spells a level and by copying scrolls and books they FIND, which is the
objects system rather than this one. **The wizard-only spells** - no
Magic Missile, no Counterspell, no Wall of Force; a Mage casting
Fireball and Banishment is a real Mage and not the whole one, and that
is the same job 102-104 did for the cleric.

**Multiclass casting is simplified and the test says so**: 5e adds the
levels on one shared table and this takes the highest casting class,
because deciding which ability the shared slots cast on is not one
answer.

## 150 broke every enrol, and the column was never called that - FIXED (152)

Found by Dave in about a minute of using the thing:

    column "prepared" of relation "character_prayers" does not exist (400)

**It was not a prayer bug. It was an ENROL bug.** 150 taught
`instantiate_character` to carry a creature's prayers across when it is
placed - right, and missing until then - but wrote them through a
`prepared` column. `character_prayers` has never had one. It has
`state`: text, NOT NULL, default `'prepared'`, and **three** values,
not two - `cantrip`, `prepared`, `book`. A cantrip is KNOWN rather than
prepared, which is why `prayers::may_reach` accepts `book` and
`prepared` and refuses to prepare a cantrip at all.

`instantiate_character` IS 123's "placing one is a copy", so this took
out every enrol, including creatures with no prayers whatsoever -
plpgsql parses the statement whether or not it has rows to copy.

**It passed migration because plpgsql resolves columns on first
execution, not at creation.** The migration applied cleanly, the commit
looked fine, and the failure waited for a user. Nothing between the
wrong word and the table: `src/*.rs` is tested, `src/commands/*` and the
SQL functions are not. The counter-pressure that exists is to run the
thing once against the live database.

**The same mistake was in the Rust, in both directions** -
`export_creature` selected `spell_key,prepared` and `import_creature`
wrote `prepared` - so a caster could be neither written to a file nor
read back from one. Nobody had hit it because nobody had exported a
caster yet; 151 had just created the first five.

**`creature_io::Prayer` now carries the word, not a flag.** A boolean
holds two of three states and silently turns the third into something
the rules have no word for - and there are live cantrip rows, so that
was a real loss and not a theoretical one. `STATES` is the list, `vet`
corrects an unrecognised word to `prepared` and warns rather than
dropping a spell the game does have, and a file may omit the field and
mean the ordinary case. Four tests, because that part is a rule.

Verified on the live database with a rolled-back probe: **14 prayers
copied, states `cantrip,prepared` both intact**, nothing created.

### Correction, 2026-10-07: 150 did not write that copy

**Checked after the fact, because the lesson depends on it.** The
account above says 150 "taught `instantiate_character` to carry a
creature's prayers across - right, and missing until then". It did not.
The copy has been there since **123**, carried forward verbatim by 124:

    123, line 165:  insert into public.character_prayers
                      (character_id, spell_key, prepared)
    124, line 160:  the same three lines again

`150_a_creature_casts_the_way_a_character_does.sql` does not contain
the string `instantiate_character` at all - it rewrote
`instantiate_npc` and nothing else. What it did was **drop the column
those older functions had been reading for a month**.

**That changes what to watch for**, which is the only reason to correct
a record rather than leave it:

| the account above | what happened |
|---|---|
| a new write path had a wrong column name | a schema change was checked against the Rust that reads it and not the SQL that does |
| the guard is "test the write paths" | the guard is "ask who else reads this column" |

Both are true and only the second one would have caught it. 150's
author - me - did grep for `prepared`, in `src-tauri/src`, and changed
every Rust reader. The functions that broke are stored in Postgres and
live in `supabase/migrations/`, which that grep never touched.

**The migration itself is left exactly as applied**, per 115: an applied
migration is a record and not a draft. This is the correction beside
it.

### The guard, which is one command

Before dropping or renaming a column, ask who else reads it - and the
SQL functions are readers:

    grep -rn "<column>" src-tauri/src supabase/migrations

`supabase/migrations` is the half that gets forgotten, because a
function body in an old migration file does not look like running code
and is exactly that. **plpgsql will not warn you**: it resolves a
statement's columns on first execution, so the migration applies
cleanly, the tests pass, the commit looks fine, and the failure waits
for whoever next presses the button.

**Run against the commit before 150, that one line returns every
reader that broke:**

    123 and 124   the two `instantiate_character` bodies
    creatures.rs  `export_creature`, selecting spell_key,prepared
    creatures.rs  `import_creature`, writing it back
    106           the column comment, harmless and a signpost

Four live readers and a signpost, in one command, before any of it
reached a user. 152 had to find and fix all four the hard way.

## The Spells tab, and Cleric Prayers dropped - BUILT (169)

161-167 took the catalogue from 108 spells to 276, **206 of them the
wizard's, and nothing in the app could show one.** Cleric Prayers was
cleric-only by design. So the Spells tab is the catalogue itself, with
the class as a FILTER rather than as the premise: search, class, level,
school, and the same `spellRow` that rendered the old tab.

`list_spells` takes an empty `class_key` to mean every class. The class
dropdown is built from `spells.classes` in the data rather than a
hardcoded list, so a game that seeds a spell naming an eighth class
gets it for free.

**`special_text` is selected now, and never has been.** That column is
where the rules are - three rays rather than one roll, what a
successful save is worth, what another slot level buys - and the
reference book was printing the flavour sentence and dropping every
mechanical rider. 473 rows carry one. **THIS IS THE THIRD IN THREE
DAYS**: 155's `character_name`, 865c8be's `prof_bonus`, now this. Three
columns that existed, were filled, and were never asked for, each
invisible until somebody noticed the screen was quietly wrong. 021 named
the shape - "an explicit column list is a SECOND schema that has to be
kept in step with the first, and nothing checks it" - and naming it has
not been enough. **A check that walks every `("select", ...)` against
the fields its caller reads would have caught all three.**

**THE TAB CAME OUT, THE SUBTAB STAYED.** Dave's call, and the line is
exactly right: the top-level Cleric Prayers tab was a reference book the
Spells tab now does better, and the SHEET's Prayers subtab is one
cleric's own list - what they hold, what they may hold, what to add -
which nothing else does.

**The loader did NOT come out with it**, and would have been easy to
delete alongside: `state.spells` is read by `paintMine` to render what
is prepared and by `paintPrepList` to offer what is not, so losing it
would have emptied a cleric's sheet SILENTLY - `byKey` returns undefined
and the render skips the row without a word. It survives as
`loadClericCatalogue`, named for what it actually does now that no tab
depends on it.

### Still open: a wizard cannot prepare anything

Found while checking what the tab was load-bearing for. **The engine has
been class-agnostic since 150** - `list_prayers`, `castable` and
`cast_prayer` all go through `sheet.caster`. The frontend never caught
up:

| | |
|---|---|
| `loadPrayers` | hides the sheet's Prayers subtab unless the character is a **cleric** |
| `loadClericCatalogue` | asks for `classKey: "cleric"`, unconditionally |
| cast panel | already class-agnostic - shows for anyone the engine says can cast |
| engine | class-agnostic since 150 |

So a wizard gets no Prayers subtab, cannot prepare, and therefore has
nothing for the cast panel to offer - while 206 wizard spells sit in the
catalogue and the whole backend is ready for them. Two stale comments
mark the spot: `list_prayers` still says "REFUSES A CHARACTER WHO IS NOT
A CLERIC" and the cast panel says "Everybody who is not a cleric refuses
this". Both describe code 150 changed.

The fix is to let the sheet follow `sheet.caster` the way the engine
does. Not taken yet.

## A spell attack is an attack - BUILT (154, spellcast.rs, character.rs)

Dave cast Spiritual Weapon to end a round and asked whether it had done
anything. It had not. The slot went, the bonus action went, the log said
**"Spiritual Weapon — +6 to hit, 1d8"**, and `rolls = 0`, `hp_events = 0`.

**Casting was always meant to be two steps and the second one did not
exist.** `cast_prayer` deliberately does not roll - `main.js` says why,
and it is right: "an attack spell is a d20 like any other and belongs in
the one place that makes them". So casting spends the slot, writes the
action and puts the spell's name in the roll box. But `resolve_request`
had no spell branch. It tries weapon/technique, then `<ability> save`,
then skill, then ability check, then **"anything else is a raw
formula"** - so the dice engine was handed the literal text `spiritual
weapon` and would have refused it. Pressing roll was never going to
work; this was not a missed button.

**The fix is a branch that returns an `attack::Attack`,** which is the
whole reason it is cheap. Everything downstream already reads that
struct: `resolution::admits` lets the roll be aimed at an AC, the crit
range decides the verdict, `a.damage` is rolled and doubled on a crit,
`a.damage_types` and `a.magical` are what the target resists, and the
hit point event falls out at the end. A spell attack that fills it in
gets all of that for free. The alternative - having `cast_prayer` roll
and write the damage itself - is less code and builds a second damage
pipeline beside `attack.rs`, which is the thing `spellcast.rs`'s header
says the module exists to avoid.

**The label and the roll cannot disagree.** Both go through
`spellcast::cast` with `sheet.proficiency_bonus()` and
`sheet.ability_mod(&caster.ability)` - the same two methods on the same
sheet - so the "+6 to hit" the cast logs is the +6 the d20 gets. One
arithmetic, two callers.

**`magical: true`, and it matters.** 143 made "resistant to bludgeoning,
piercing and slashing from nonmagical attacks" expressible and that is
the commonest resistance in 5e. A spectral mace is magical by
definition, so a werewolf does not halve it.

**`damage_types` is empty and that is a real gap.** `spells` has no
damage type column - Inflict Wounds is necrotic, Guiding Bolt is radiant,
and neither can say so - so a target's resistance does not apply at all.
The `Attack` struct's own comment calls an empty list "the safe way to be
ignorant": understated rather than wrong. **It wants a column, not a
rule.**

**Only Attack spells resolve here.** A Save spell is the TARGET's roll
and the engine does not roll for somebody else's character, which is why
casting one reports the DC and stops; a Utility spell rolls no d20 at
all. Three spells in the catalogue are `cast_type = 'Attack'`: Guiding
Bolt, Inflict Wounds, Spiritual Weapon.

**The key is `attack`, not `spell_attack`** - the same decision the save
branch records a few lines below. The key is the vocabulary
`narrative_lines` and `skill_prompts` are written in, and a key nobody
has seeded silently loses a character their prose.

### Two queries, paid by clerics and nobody else

`load_sheet`'s own note says seven queries "is the count to watch" and
that a hover preview pays all of it. The prepared list makes it nine -
but only when `prayers::caster` says this character casts, which is
computed from class rows already read. **A fighter, a monster and every
creature in the bestiary still pay seven.**

Two and not one because `character_prayers.spell_key` references
`spells` BY VALUE - the nullable-tenancy pattern, where a partial unique
index cannot back a foreign key - so there is no relationship to embed
across and the keys must be known before the catalogue can be asked.

### The effect belonged to the wrong creature

Dave's call, and he talked himself out of the alternative on the way:
putting it on the encounter would break for a long spell that outlives
the fight.

The chip went on **Webbys** - the thing Luci was hitting - because
`cast_prayer` applied the effect to `target_character_id`
unconditionally. That is right for Bless and wrong for a floating weapon
the caster maintains. Worse, its `grants` was `[]`, so it sat there for
ten ticks doing nothing: the silent-no-op shape again.

**Routed on `cast_type = 'Attack'`, not on stance.** Stance lumps Attack
in with Save, and the lasting Save spells are mostly debuffs that
genuinely DO sit on their victim - Hold Person, Bane, Blindness, Bestow
Curse. Routing by stance would have moved all nineteen onto the caster
to fix two.

**The rough edge that remains, named rather than fudged:** Spirit
Guardians, Blade Barrier and Guardian of Faith are Save spells that hang
around the caster, and they still land on the target. `spellcast.rs`
already records that the four-word vocabulary cannot tell Zone of Truth
from Bane. Same gap, same answer: it wants a column.

## A cleric can heal somebody - BUILT (155, 156, 157, death::healed)

**Seven healing spells carry dice and not one of them reached a hit
point.** Cure Wounds spent a slot, logged a line and changed nothing. It
never showed because resting heals (`time.rs`) and nobody had tried it
mid-fight.

**The cause was structural, not a missing case.** There is exactly ONE
write to `hp_events` in the whole roll path - `"delta": -total`, inside
`if let Some(a) = &resolved.attack`. Every route to hit points ran
through a weapon-shaped Attack, so 154 making spell ATTACKS work was the
same fix arriving from the other side, and a heal is not an attack.

### `death::healed` - the dice and the delta are not the same number

013 made current HP the maximum plus the sum of signed deltas, and
`hp_floor` clamps what is SHOWN at zero while the sum underneath keeps
going. **Falon sat at a raw -1 of 43 and read 0.** A flat +8 would have
put him on 7, the heal spending its first point climbing out of a hole
5e says is not there.

    healed(hp_max, summed, amount) -> the delta to write

Verified live, rolled back: `falon -1/43 -> 8`. Five tests, including
healing from a -30 hole and healing somebody already full (zero, and the
caller writes no event at all - 013's log records what CHANGED).

### Two faults in `write_action`, and one of them was mine

**It dropped `cost`.** The INSERT names its columns and `cost` was not
among them, so a caller could not say what an action costs.
`stamp_action_cost` fills a null from the key, and every caller so far
wanted what it derives - but a spell is where that breaks: Healing Word
and Spiritual Weapon are BONUS actions with key 'spell', so the trigger
would say "action" and quietly take a cleric's whole turn. That is why
`cast_prayer` wrote its own action row and so could never write a heal
atomically with it. **This is 021's lesson word for word** - "an explicit
column list in a writer function is a SECOND schema" - same function,
different column.

**157 is 156's bug.** 156's header stated that nothing constrained
`rolls.role`. `rolls_role_check` allowed only `to_hit`, `damage` and
`check`, so the first heal would have failed the insert and taken the
whole cast with it. **A rolled-back probe found it, not Dave** - calling
`write_action` with a heal row inside a transaction that raises at the
end. Checking a claim about the schema by ASKING THE SCHEMA costs one
query; believing it costs a session. 156's sentence is annotated in place
rather than rewritten, per 115.

### 155 - "Someone" was the log asking the wrong row

Every cast read `Someone`, because the log takes the name off the to-hit
roll and **a cast writes an action and no rolls** - that is 110's
two-step design. The action has known `character_id` all along.

**A trigger, not a column in `write_action`.** 021 had to add
`character_name` to the writer as well as the table, because that
function drops what it does not list. A BEFORE INSERT trigger needs
nothing from the writer, so the name cannot be lost in transit by
`write_action`, by `cast_prayer`'s REST insert, or by whatever writes an
action next. `write_action` was deliberately not touched by 155.

**And `character_name` had to be added to the log's SELECT** - the same
trap one layer up. A column the database fills and the query never asks
for does not exist as far as the screen is concerned.

## A save spell lands - BUILT (158, resolve_spell_save)

The last of the three shapes. An **Attack** spell rolls a d20 (154), a
**Heal** rolls at cast time (156), and a **Save** spell is the one where
the caster names a number and somebody else rolls against it.

**The DM asks for the roll, which is what makes it allowable.** The
engine's stated position is that it does not roll for somebody else's
character, and that still holds: casting Sacred Flame reports "DEX save
DC 14, 1d8" and stops. A button appears beside the target's name and the
roll happens because the DM pressed it - the same distinction as a DM
rolling a monster's save at the table.

Then it is the chain an attack already has: one d20 judged against a
number, the dice that follow from the verdict, and an action, its rolls
and one hit point event through `write_action`, together or not at all.

**It costs the saver nothing.** `stamp_action_cost` would have derived
"action" from the key and quietly eaten the target's turn. 156 added
`cost` to `write_action` an hour earlier for the heal path, and this is
the first caller that actually needed it - `cost: 'free'`.

**The Ny'ook bonus reaches it for free.** The request is built as
`"<ab> save vs spell"`, and 098's against-a-spell bonus has always been
read off those two words. This is the one caller that can say so without
a human remembering to type it.

### Three of the eleven were never damage, which is the trap 158 exists for

`spells.dice` cannot tell Fireball from these:

| spell | dice | what they actually are |
|---|---|---|
| Bane | 1d4 | the penalty on attacks and saves - a GRANT, which 114 already runs |
| Bestow Curse | 1d8 | extra necrotic on one of four curse options, conditional and later |
| Geas | 5d10 | psychic each DAY the target disobeys, not on the save |

A rule that damaged on every failed save would have Bane hurting someone
as well as cursing them, and Geas killing a creature for failing a save
it was always going to fail.

**`on_save` says what a success is worth, and NULL says "not damage".**
`'half'` (5e's usual), `'none'` (Sacred Flame), NULL for everything
unmarked. **NULL is the default on purpose**: an unmarked spell does
nothing rather than guessing, because a DM can see a spell that did
nothing and cannot see one that quietly did the wrong thing to a player.
The eight that do damage were read off their own prose, which states it
in every case.

### Verified, rolled back, against live data

    cost=free  name=Webbys  rolls=2  delta=-6
    linked to the damage roll   Webbys 18/26 -> 12

And the constraint check that the last round taught: `rolls.target_kind`
already allows `'dc'`, so the save row records what it was trying to beat
(009) rather than a bare number.

### Still open

**Spell damage has no type.** `spells` has no `damage_types` column, so
none of it - attack, save or otherwise - can be resisted. Everything
else about resistance has been built since 116; this is the one column
standing between it and spells.

**Flame Strike is understated.** The book is 4d6 fire AND 4d6 radiant;
`dice` holds 4d6 and there is nowhere to put the second half. Same
missing column.

## The Lich's DC was 19 and should have been 16 - FIXED (159, 160, load_profile)

Dave cast Hold Person with a Lich, Fireball came back greyed out, and
the log turned up two faults that had nothing to do with each other and
nothing to do with spells.

### `prof_bonus` had never been read, by anything, since 022

`load_profile`'s select did not include the column. `c.get("prof_bonus")`
returned None every time, so `Sheet.prof_bonus` was always None and
`proficiency_bonus()` fell to the level formula - for the Lich,
`((18-1)/4)+2 = 6`, so **8 + 6 + 5 = 19**.

The field's own doc comment says "STATED rather than derived, for a
monster... a statblock simply has one". It has never once been used. The
column exists, `instantiate_npc` copies it faithfully, and nothing
selected it. **130 of 280 statblocks** had a stated value that differed
from the derived one, and every one of them had been hitting and saving
better than its statblock said.

**It stayed invisible because the two scales agree at the bottom.** A
Giant Spider is 2 either way. It only diverges above level 9, and until
150 no creature with a level that high could cast anything - so the
first number big enough to notice was a spell save DC.

### 159: two spells with no casting time

`spellcast::cost` maps anything that is not an action, a bonus action or
a reaction to `TooLong`, and NULL is not any of those. Exactly two
spells had no casting time - Fireball and Aura of Life - and both come
from 105, the pair rescued off ONE CHARACTER'S SHEET rather than written
from a list. Everything 102-104 seeded has one.

**Fireball had never been castable in a fight, by anybody, since 105.**

A default would have hidden it: making `cost` treat NULL as an action
is right almost always, and a spell seeded without a casting time would
then look correct forever instead of being visibly wrong the first time
somebody reached for it. `TooLong` is a bad answer that announces
itself.

### 160: one proficiency scale, at last

147 left the old 203 on 127's two-step rule and wrote that deriving them
from CR was "a one-line change whenever Dave wants it". This is that
change, **taken on purpose rather than as a side effect**, which is the
whole difference from 129.

**It could not be derived from `level`** - the same trap 151 nearly fell
into with caster level. `npcs.level` is 127's hand-set encounter weight
for 130's creatures and the CR only for 147's, and **the two differ on
179 of 261**. Deriving from it would give the Lich a 6 where the book
says 7. So the CR was read off the published SRD, one page at a time,
and **`cr` is now a column** - a derivation nobody can check is not much
better than a guess.

| | |
|---|---|
| +4 | Lich 3 → 7 |
| +3 | Pit Fiend, Balor, Adult Red Dragon |
| +2 | eleven, including four adult dragons, Iron Golem, Vampire, Storm Giant |
| +1 | thirty-two |
| −1 | Black Pudding, Chuul, Red Dragon Wyrmling, where 127's rule was too generous |

**The top end was worst affected**, which follows: 127's rule stopped at
3 and the book goes to 9, so everything legendary had been swinging like
a mid-level monster. **211 do not move**, which is the quiet evidence
that 147's 77 were right to begin with.

**Nineteen are left alone with `cr` NULL** - the six we invented and the
thirteen written from memory of a book they are not in. Inventing a CR
to derive a proficiency from would be two guesses stacked, and NULL says
"nobody has rated this" where a number would not.

### What these left behind

**`npcs.level` is still two scales.** 130's creatures carry 127's weight,
147's carry CR, and they disagree on 179 rows. Not touched here: level
gates techniques and 132 had to put those back once already. Now that
`cr` is stored the two can be compared instead of confused.

**Proficiency is still stored rather than derived.** The honest shape is
`cr` stored and `prof_bonus` computed in Rust where it can be tested -
001's "derive what can be derived". That is a change to
`Sheet::proficiency_bonus` and its callers, and belongs in its own
commit.

## The scribe's rules - BUILT (170, scribe.rs). STORAGE IS DAVE'S CALL

Dave: *"develop the spell book and scroll rules 1st ... the actual
physical spell book(s) item, pens, ink, time, cost, special materials
... then the rules for scribe time per level and spell type."*

**The rules and the things are built. Where the writing GOES is not,
and that is the one decision left - see the fork below.**

### The cost is the materials, not a number beside them

The book charges *2 hours and 50 gp per spell level*, and says the 50 gp
is the fine inks and components burnt through working the spell out. So
nothing charges 50 gp. It consumes **ink**, and the ink is what costs:

    two vials per spell level, at 25 gp a vial = 50 gp per level

A ninth-level spell wants eighteen vials and 450 gp - **the published
figure, arrived at by buying something real.** A wizard who already has
ink does not pay twice. A wizard with a full purse and no ink in a
wilderness cannot copy anything, which is the whole reason to make it a
thing instead of a price.

**171 set the campaign's rates and they are SILVER** - Dave's campaign
is not gold heavy. The ratio did not move, only the price of a vial,
which is 170's decision paying off: the economy is rows in `items`, not
a constant in Rust.

| | | |
|---|---|---|
| `ink_vial` | **5 sp** | consumed, two per spell level |
| `quill` | 2 sp | a tool, required, not consumed |
| `scroll_blank` | **10 sp** | vellum with nothing on it yet |

So a 1st-level spell is **10 sp** and a 9th is 90 sp.

### Four books, measured in spell levels

| | | | | |
|---|---|---|---|---|
| Adventure Spell Book | 20 sp | sm | 2.5 lb | **10 levels** |
| Acolyte's Spell Book | 30 sp | med | 5 lb | **20 levels** |
| Mage's Spell Book | 50 sp | med | 5 lb | **30 levels** |
| Tome of Spells | 100 sp | med | 10 lb | **50 levels** |

**Levels, not spells** - a Tome holds five ninth-level spells or fifty
cantrips, and the unit is what makes those different. `scribe::pages`
is `max(1, level)`: a cantrip takes a page like everything else,
because level 0 would be free and a book would hold infinitely many.
That floor is this project's call, not Dave's.

**`capacity_slots` could not carry it**, which corrects 170. That column
is the CONTAINER system's volume measure - what 036 walks to decide a
backpack is full - and spell levels are not a volume. 170 reused it and
called it "the page count now", a pun that would have collided the first
time somebody put a real object in a book. `items.spell_levels` is its
own column and the books hold no gear.

`spell_book` has been in the catalogue since 027 and **nothing has ever
owned one**, so it was free to reshape into the Mage's rather than being
deleted and leaving a key that once meant something.

### Per level and per school, and the school part is data

`scribe::to_copy(level, school_pct)` - hours and ink from the level,
bent by a percentage and **rounded up**, with a floor of one of each so
a cantrip is never free. `scribe::to_scroll` is twice that, and is a
house rate: the SRD prices scrolls as treasure and leaves writing one to
the DM.

`scribe_schools` is the tuning, **seeded at 100 with no reagent for all
eight**. That is deliberate and is not the same as not having built it:
100 is the only rate anybody can defend yet, and which jar a conjurer
empties is a question about Dave's world rather than about 5e. Tuning is
eight UPDATEs, not a Rust change.

`scribe::may_copy` refuses four ways, each a sentence a player can act
on: not a book caster, not on their list, already written, and **above
what they could prepare** - a 3rd-level wizard cannot bank a 9th-level
spell out of a captured book. 13 tests.

### `spell_book.capacity_slots` is the page count now

027 made `spell_book` a container with `accepts = {spell}` and
`capacity_slots = 10` - an intention nothing ever implemented, and no
item has ever been tagged `spell`. One spell now takes one slot, so the
standard book holds ten and a wizard who wants more buys another book.
Reuses the column rather than adding a second idea of capacity, and
makes "spell book(s)" plural for a reason.

### THE FORK: where do a book's contents live?

**This is the decision, and it is not 50/50 - the current model cannot
do what was asked.**

**Today (150):** a wizard's book is `character_prayers` rows in state
`'book'`. `may_reach` enforces "a wizard prepares only from what is
written down" and is tested. But those rows belong to a CHARACTER, so:

- a spellbook **found in a dungeon** or taken off a corpse cannot have
  contents - there is no character to hang them on
- **a second book means nothing** - no row says which book a spell is
  in, so the capacity rule above has nothing to attach to
- **a scroll cannot exist** - it is an unowned object with a spell on
  it, which is the same problem from the other end

Scrolls alone force the change, and solving scrolls separately from
books would be the same fact in two places.

**The fix: contents belong to the OBJECT.** One table -
`scribed_spells(object_id, spell_key, scribed_at)` - and a book and a
scroll become the same story: *a spell written on a physical thing*. A
book is an object that holds several up to its capacity; a scroll is one
that holds exactly one and is consumed.

**`may_reach` does not change.** It already takes `held: Option<&str>`
and lets the caller decide; only the lookup that feeds it moves, from
"rows on this character" to "spells in books this character is
carrying". `character_prayers` keeps `prepared` and `cantrip`.

What it buys, all from the one table: found books, stolen books,
lending, a second book, real capacity, and scrolls.

**Not taken, because it moves where a wizard's spells live and that is a
one-way door.** Everything above is useful under either answer, which is
why it was built first.

## Pick up here

**Be clear about what is and is not done.** The foundation is square
and the combat loop runs: the access model, the dice, the sheet
resolver, the prose, equipment, attacks, hit points, dying, the DM's
screen, and monsters that are individuals rather than views of a type.

Inventory is now real on both sides: a catalogue to pick from, a
designed way to acquire, and objects that can be named, split, dropped
and destroyed.

**As of 2026-10-03** there is also a game clock, rests that spend and
return hit dice, feature uses with recharges, effects that expire on a
tick, the cleric spell list with prepared prayers and slots, casting
that lands on a target and holds concentration, enchantment as one grant
vocabulary that reaches the dice, damage resistance with its provenance,
and saves you can roll from the ability row. The UI is no longer a test
rig: seven tabs and five sheet subtabs, and nothing on screen computes a
game number.

**As of 2026-10-06** a creature also knows WHAT IT IS - 5e's fourteen
types, on species, statblocks and characters, so the fifteen spells that
name a type can be checked. A monster says what it resists in the same
grant vocabulary everything else uses. A statblock can be instantiated
as a template character and copied into an encounter with its kit, which
is how two goblins off one statblock get their own hit points. A
creature can be written to a file and read back. And the global
catalogue holds 134 statblocks carrying 223 kit rows, 45 of them with
resistance grants. **Spot-check a creature before a session runs on
it:** those numbers are SRD-derived from memory and were verified as the
rows intended, not as the numbers printed.

Since the last handoff: an object edit now reaches the sheet and not
just the Objects tab (five other object writes had the same staleness);
the encumbrance walk applies an object's own `weight_override` instead
of the catalogue's, which had Hapi showing 25 lb on the item and 19 lb
on the carry line from the same row; and `items.description` is filled
on all 89 catalogue rows - the column had existed since 004 and nothing
had ever written to it until 070 put it on screen. Characters can be
renamed, can be more than one class, and a bard can play to a room.

**What AppSheet did that this still does not is *deliver*.**
`rolls.status` goes `pending -> resolved -> delivered` and nothing in
this codebase moves a row to `delivered`. There is still no die art, and
the session still lives in memory - a PIN saves you retyping a password,
but a restart signs you out and persisting it wants the OS keychain
rather than a file.

**The migration chain is complete, and was not on 2026-10-03.** Seven
files - 101 to 107, the spell catalogue and the whole cleric list - had
been applied to the live project and never committed, the same fault
0ed7491 fixed for four others. They were recovered from
`supabase_migrations.schema_migrations`, which keeps the statements it
ran with the comment headers intact, and every one was MD5-verified
against that record, so they are the files AS APPLIED.

**The 19 remaining gaps are not missing anything.** Every change gets a
number; only a change that touches the schema gets a migration. 070
taught the sheet to show a description and 071 filled the column; 099
is an ability cap in `species.rs`; 111 and 112 gave casting a target in
JavaScript and Rust. Checked against the database's own record: every
numbered migration it has, this folder has.

The habit that caused the gap was applying first and writing the file
afterwards, which in practice means never. Write the file and apply it
in the same breath. `tools/recover_migrations.py` makes the recovery one
command if it happens again.

**And the second migration habit, added 2026-10-07 after 150 broke every
enrol: a column has readers in two places.** Before dropping or renaming
one, `grep -rn "<column>" src-tauri/src supabase/migrations` - both
halves. A function body stored in Postgres is running code that does not
look like running code, and plpgsql resolves its columns on first
execution rather than at creation, so nothing fails until a user presses
the button. See the correction under 152.

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
10. A real phone-first UI. What exists is a working desktop UI - seven
    tabs, five sheet subtabs, and every number on it coming from the
    engine - but it is laid out for a wide window. The left column sits
    around 550px in practice, which is already tight enough that the
    ability rows wrap their save half onto a second line.
11. Android via `npm run tauri android init`. iOS needs a Mac.

Two small things worth doing while they are cheap: `preview_request`
calls `load_sheet`, so hovering a button reads the whole pack it has no
use for - a `load_sheet_lite` would fix it, at the cost of two loaders
that can drift. And `load_sheet` runs on every roll, so the pack read
sits ahead of the dice; it is one small select and it has not been worth
fixing yet, but that is where the latency is if it ever matters.

**That cost has grown and is worth watching.** `load_sheet` now also
reads the passive class features and the live effects, and 116 made the
damage path load the TARGET's whole sheet to find out what they resist.
That was deliberate - "the same answer the sheet would give", and a
second cheaper loader would be a second answer to one question, which is
the fault this codebase keeps producing. But a swing against a creature
is now a full sheet read on both ends. If a fight ever feels slow, that
is where to look first, and the fix is caching a sheet for the length of
a turn rather than writing a second loader.

## Loose ends

- **DELETING A GAME IS A TWENTY-FOUR TABLE CASCADE, AND NOTHING WARNS
  YOU.** Every tenanted table points at `games` with ON DELETE CASCADE
  - characters, rolls, actions, hp_events, objects, entities,
  locations, encounters, effects, and the per-game rows of every
  reference table. One delete takes a campaign's entire history with
  it, silently, in one statement.

  IT HAS ALREADY HAPPENED ONCE. Between 2026-10-01 and 10-02 three of
  the four games went, and with them 16 characters, ~130 rolls, 70
  actions, 34 hp_events, the Inn's location tree and four encounters
  including the Baseline Test fight. It may well have been deliberate
  housekeeping before the chargen work - it is recorded here because
  nothing in the app or the database would have told anybody either
  way. What survived is `Test Game 1`, Falon (fighter 4 / bard 1, the
  same row) and Goblin 0001. Reference data was untouched: the
  catalogue rows are global, with `game_id` null.

  THE CASCADE ITSELF IS RIGHT. A game's data belongs to the game and
  orphaning it would be worse. What is missing is everything around
  it:

      no confirmation      no command asks twice, or at all - the
                           deletes so far have been by hand in SQL
      no archive           053 chose `archived` over deleting an
                           ENCOUNTER because rolls point at it. The
                           same argument applies with more force to a
                           game, and games never got the treatment
      no backup            see below - nothing in git holds play data

  The obvious shape is 053's, one level up: `games.archived`, and a
  delete that refuses while any roll in that game exists. Not built,
  and worth building before a second person is at the table.

- **NOTHING BACKS UP THE PLAY DATA, and the schema is not the
  problem.** The migrations are the schema's backup and now genuinely
  are again (see the drift trap). What exists nowhere but the live
  database is what people DID: characters, rolls, actions, hit point
  history, objects, locations. The whole database is ~15 MB, of which
  the catalogue - items, techniques, class features, species - rebuilds
  from migrations and is not worth saving.

  Before anything destructive, and periodically:

      supabase db dump --data-only -f backup-YYYY-MM-DD.sql

  It wants the CLI and the database password. Whether this project's
  plan includes automatic daily backups has not been checked - it is
  Database -> Backups in the dashboard, and worth knowing before
  relying on it rather than after.

  TODAY'S MISSING MIGRATIONS WERE NOT A BACKUP PROBLEM and a backup
  would not have caught them. Keep the two apart: git holds the shape,
  a dump holds the contents, and each is useless for the other's job.

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
- **SIX registered commands have no caller**, out of 104. An uncalled
  command is a defect rather than a spare - and this entry said "one"
  until the whole list was audited instead of the last count being
  trusted:

      take_object       the Take rule in acquire.rs is the access
                        control work it belongs to
      quote_object      040's pricing, reachable from nothing
      list_members      who is in the game, never shown
      rename_location   both added by c7c26ea, neither ever wired to
      move_location     a control on the World tab
      roll_dice         the formula sandbox; useful, unreferenced

  Five others were settled earlier on the same rule:
  `set_encounter_location` and `destroy_object` got the screens they
  were waiting for, and `loose_objects`, `load_techniques` and
  `list_roster` were deleted. Wire or retire, one at a time.
- **A STORED SCORE IS NOT AN EFFECTIVE ONE, AND FOUR PLACES READ THE
  WRONG ONE.** 056 keeps a people's bonus out of
  `character_abilities` deliberately, so the row holds what somebody
  rolled and the character has something else. Every site that turned
  a score into a modifier had to know that, and each had been written
  before there was anything to know:

      the target list    AC off the stored DEX, and None where the
                         species floor goes, so an unarmoured
                         Unt'garoth reads 10 + DEX on the list a
                         goblin's attack resolves against and 12 + CON
                         on their own sheet
      the level button   Constitution off the row, so levelling an
                         enrolled character recomputed their maximum
                         five points low, over what rederive_hp_max
                         had got right
      initiative         DEX off the row; latent only because neither
                         seeded people raises DEX
      rederive_hp_max    correct, and a second copy of the resolution,
                         which is how the other three stayed wrong

  NO LIVE VICTIM, and the first account of this said there was one.
  Garn was cited as reading 14 on his sheet against 10 on the target
  list. He wears CHAIN MAIL - base 16, dex cap 0 - and body armour
  beats the unarmoured floor, so both screens said 16 before the fix
  and both say 16 after it. The claim came from a query written
  against `objects.holder_id = characters.id`, which answers zero for
  everybody: since 031 a holder is an ENTITY and the join is through
  `characters.entity_id`. A wrong join returning no rows reads exactly
  like a character with no armour.

  The defect in the code was real and is worth the fix - the target
  list plainly passed None and plainly read stored scores. What it did
  not have was a character it was wrong about yet.

  `character::load_effective` is the one loader now: stored rows
  raised by each character's people, three requests for any number of
  them, with `modifier()` and `unarmored()` on the result. All four
  call it. `load_sheet` does not, only because it is already reading
  everything for one character - it reaches the same answer through
  the same `apply_species`.

  Character creation also resolves its own Constitution, and is left
  alone on purpose: it has just read the species to grant skills, so
  asking the loader would re-read what it is holding. It calls the
  same `species::effective_score` the loader does.
- **A POSTGREST EMBED NEEDS EXACTLY ONE RELATIONSHIP, AND 051 ADDED A
  SECOND.** `encounter_actors.encounter_id` has pointed at
  `encounters` since 011; 051 added `encounters.turn_actor_id`
  pointing back. Two paths, so `encounter_actors?select=encounters
  (game_id)` is refused outright - "more than one relationship was
  found", a 300 - and it is refused at RUN TIME, on a table pair that
  was embeddable when the older half was written.

  The good news is that it says so. Compare the two selects traps
  above, which both produced a calm wrong answer. The fix was to ask
  through `characters(game_id)`, the one unambiguous hop, rather than
  to name the constraint - a constraint name in a query is a schema
  detail no migration is obliged to keep.

  **AND THAT FIX LASTED UNTIL 139.** `encounter_actors.template_id` is
  a second reference to `characters`, so the "one unambiguous hop" went
  ambiguous too, and `roll_initiative` started returning the same 300 -
  found by Dave on 2026-10-06, one click after the 152 enrol fix let him
  get that far. The reasoning above was wrong, and worth keeping
  because of HOW it was wrong: it treated "this pair has one
  relationship today" as a property worth building on, when the thing
  that actually moves is the schema. Choosing a hop is choosing a fact
  that any later migration can falsify without knowing it has.

  **153 NAMES THE CONSTRAINT INSTEAD**, in both places that embed
  `characters` from `encounter_actors` -
  `characters!encounter_actors_character_id_fkey(...)` in
  `commands/initiative.rs` and `commands/log.rs`. The original
  objection stands - a constraint name IS a schema detail - but it is
  the better risk of the two. A name only breaks if somebody renames or
  recreates that FK; a hop breaks whenever anybody adds any FK at all,
  which is a thing that happens on a normal Tuesday. Both failures are
  loud, so the question is only which one happens less.

  PostgREST hands you the fix in the error: the `hint` lists the exact
  spellings, and the `details` name both relationships. Read it rather
  than guessing which hop is clean.

  WORTH CHECKING BEFORE ADDING AN EMBED: whether the other table
  points back - and prefer the named constraint from the start.
  `actions -> rolls` in `commands/log.rs` is still a single path, which
  is a fact about today and not a guarantee.
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
