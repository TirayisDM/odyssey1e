# A path through the whole application

**This is the document for somebody who has it running and does not
know what it does.** `README.md` builds it, `NEW-MACHINE.md` stands it
up, `MAP.md` and `ART.md` say where the map is going, and `STATUS.md`
remembers every cycle. None of them tells you what to click. This
does.

**IT IS A SEQUENCE AND NOT A FEATURE LIST,** on purpose. A list gets
read once and a sequence gets run, and almost nothing here is reachable
cold: Trade does nothing until somebody is a merchant, the Spells tab
on a sheet is hidden until the character can cast, the fight strip is
not drawn until initiative is rolled, and the dice pad says *no d20
rolled yet* until one is. Walking it in order is the shortest way to
see all of it, and the order is the dependency order rather than a
tour guide's taste.

**WHAT PROVES IT IS PART OF EACH STEP.** A step that has stopped
working then fails where you are standing instead of three screens
later, and you can say which one it was.

## How to read a step

| column | what it is |
|---|---|
| **do** | the tab path, then the control, labelled as the screen labels it |
| **proves it** | what you should see. If you do not see this, stop here |
| **lives in** | the element id and the Tauri command, for reading the code |

The **lives in** column is why this file is also useful for examining
rather than only for using: when a step misbehaves, the command name is
where to start in `src-tauri/src/`, and the id is where to start in
`src/index.html` and `src/main.js`.

**`tests/tour.smoke.mjs` checks that column.** Every `#id` in this
file must exist in `src/index.html` and every `command()` must be
registered in `src-tauri/src/lib.rs`. That is the whole test, and it is
worth being exact about what it does and does not buy you:

- it catches a control that was renamed or deleted and a command that
  was unregistered, which is how a document like this normally dies
- **it does not catch a step whose order is wrong, or whose "proves
  it" is no longer what happens.** Nothing mechanical can.

**AND NOBODY HAS WALKED THIS END TO END.** It was derived from the
markup and the handlers and checked by that test. The first person to
walk it will find something, and the right thing to do with it is fix
this file in the same breath.

---

## 0. Before you start

Clone, install the prerequisites, `.\run.ps1`. All of that is
`NEW-MACHINE.md` sections 1 and 2, and none of it is repeated here.
You need nothing secret: the only credential in the repo is Supabase's
*publishable* key, which is designed to ship in clients.

Two things to know before the first screen makes sense:

- **The app opens on a door, not on the tabs.** It waits for `me` and
  `pin_status`, shows a sign-in, and then **makes you pick a game
  before anything is drawn.** There is no last-game memory; that is
  deliberate (275-284).
- **There are three tabs** - `Active`, `Campaign`, `Rules` - and the
  right-hand column is the narrative and roll feed, open by default,
  with the developer log as its second tab.

---

## 1. Getting in

| # | do | proves it | lives in |
|---|---|---|---|
| 1.1 | Sign up with your own email and a password | you land on the game chooser with no games | `#signup` `sign_up()` |
| 1.2 | **Create game**, name it anything | the game name appears top left beside the mark, and the tabs draw | `#create-game` `create_game()` |
| 1.3 | Click the game name top left | the chooser reopens and lists it | `#refresh-games` `list_games()` |
| 1.4 | Menu under your name, **Save PIN**, four digits | it says the PIN is set | `#save-pin` `set_pin()` |
| 1.5 | Sign out, relaunch, enter the PIN | you are in without typing the password | `#do-unlock` `unlock()` |
| 1.6 | Menu, **Forget PIN** | the next launch asks for the password | `#clear-pin` `forget_pin()` |
| 1.7 | Menu, **Change password** | it accepts the old and new and says so | `#show-change` `change_password()` |

**THE PIN IS CONVENIENCE AND NOT SECURITY,** and `pin.rs` says so in as
many words. It stores a refresh token in a file beside the app;
anybody who can read that file can mint a session without knowing the
PIN. Do not set one on a machine you do not control.

**Nothing in this application can read or reset a password.** A
locked-out account needs a reset from the Supabase dashboard by
whoever holds the project.

---

## 2. A character

The whole of `Active > Characters > PCs`. This is the longest part of
the tour and everything after it needs a character to exist.

| # | do | proves it | lives in |
|---|---|---|---|
| 2.1 | **+ New character** | the builder opens below the roster | `#new-char-toggle` |
| 2.2 | **Roll 3d6 (reroll 1s) x7** | seven numbers appear - seven for six slots, so one is dropped | `#roll-stats` `roll_ability_spread()` |
| 2.3 | Assign them, then **Lock abilities** | the spread is fixed and the class and species pickers accept | `#lock-abilities` |
| 2.4 | Pick a species | a note appears saying what that people adds | `#char-species` `list_species()` |
| 2.5 | Pick a class | a note appears saying which die the hit points come from | `#char-class` `list_classes()` |
| 2.6 | Name them, **Create character** | the roster gains a row, the list folds, the sheet opens | `#create-char` `create_character()` |
| 2.7 | Read the **Stats** pane | a round portrait slot, the name, class, abilities, saves | `#sheet-panel` `get_sheet()` |
| 2.8 | **Set level** to 3 | hit points change and the class table grants what level 3 grants | `#save-level` `set_level()` |
| 2.9 | **+ add class** | a second class row, with its own level | `#add-class` `set_class_level()` |
| 2.10 | **Set name** | the roster row and the sheet heading both change | `#save-name` `rename_character()` |
| 2.11 | Drop or paste an image on the portrait | it uploads and becomes the default picture | `upload_character_image()` |
| 2.12 | **Description**, *start from the people's words*, then **Save description** | the species' own prose fills the box, then persists | `#desc-from-species` `set_description()` |
| 2.13 | **Skills & Talents**, **edit**, tick some, **Save** | the skill rows show the proficiency bonus added | `#save-profs` `set_skill_prof()` |
| 2.14 | **Equipment**, **Add** an item from the catalogue | it appears in the pack with a weight | `#add-item` `list_inventory()` |
| 2.15 | Equip it to a slot | the slot fills and the armour or attack numbers move | `set_item_slot()` |
| 2.16 | **check keys** | it reports whether every item's key resolves against the catalogue | `#check-keys` `check_item_keys()` |

**SEVEN FOR SIX SLOTS IS 066,** and the roll happens on this device
rather than on the server, which is the one place in the application
where that is true and the comment there says why.

---

## 3. The reference shelves

`Rules`. All of it is global catalogue data - `game_id IS NULL` -
readable by any signed-in account, so a brand-new game opens onto the
full reference with nothing borrowed from anybody.

| # | do | proves it | lives in |
|---|---|---|---|
| 3.1 | `Rules > Species` | the playable peoples, with a count in the tab | `list_species()` |
| 3.2 | `Rules > Spells`, filter by class, level and school | the count shrinks as you filter | `#spell-find` `list_spells()` |
| 3.3 | `Rules > Items` | the catalogue, with weight, cost and what it grants | `list_catalogue()` |
| 3.4 | `Rules > Creatures` | the bestiary. **This tab is the DM's** | `#creature-list` `list_creatures()` |

**SPECIES IS BESIDE CREATURES AND NOT INSIDE IT** (279), because it is
the most player-facing reference in the application and it spent a long
time inside a panel that hides itself from anybody who is not the DM.

---

## 4. The world, and a place to stand

`Campaign > World`. Nothing in the fight or the map section works
until there is somewhere for it to happen.

| # | do | proves it | lives in |
|---|---|---|---|
| 4.1 | **Add place**, kind *region*, no parent | it appears at the top of the tree | `#create-location` `create_location()` |
| 4.2 | **Add place**, kind *room*, parent the region | it nests under it | `#loc-parent` `list_locations()` |
| 4.3 | Select the room, `People` | who is standing here, which is nobody yet | `who_is_where()` |
| 4.4 | **Bring here** with your character | they are now standing in the room | `#bring-here` `move_character()` |
| 4.5 | `Things`, **Place** an item from the catalogue | it is lying in the room, belonging to nobody | `#stock-here` `stock_here()` |
| 4.6 | Pick it up from `Active > Things` | it moves from the floor into the pack | `drop_here()` |

---

## 5. A map

`Campaign > Maps` for the library of them, and the one the party is
standing on is in `Active`. A map is a fact about a **place** (235/279),
so it hangs off the room you just made.

| # | do | proves it | lives in |
|---|---|---|---|
| 5.1 | Pick the room in **where**, set width and height, **Draw** | a grid of squares appears on the canvas | `#mv-draw` `set_map()` |
| 5.2 | Mode **Edit**, pick a surface, paint some squares | the tiles change and stay changed after a reload | `#mv-mode` `paint_tiles()` |
| 5.3 | Paint a wall, then **Inspect** it | it reports the surface, the sub-surface and the height | `#mv-say` `inspect_square()` |
| 5.4 | Mode **Play**, **Hit it** on the wall | the wall takes damage and a heap of rubble lands on the square | `#mv-strike` `damage_square()` |
| 5.5 | **Lift what is underfoot** | it lifts - *given a shovel, spade or mattock in the pack.* Otherwise it tells you what it wants | `#mv-haullift` `take_rubble()` |
| 5.6 | Place your character, then click a square away from them | they walk, and it says how far the move went | `#mv-at` `walk_to()` |
| 5.7 | **Describe** a square | plain words for the terrain rather than keys | `#mv-describe` `terrain_words()` |
| 5.8 | **Set** hit points on a square, then **Clear** | the square carries its own durability | `#mv-sethp` `set_square_hp()` |
| 5.9 | **Save as** a version name | it appears in the version list | `#mv-save` `save_map_as()` |
| 5.10 | Break something, then **Load** that version | the map is back as it was | `#mv-load` `load_map_version()` |

**THE RUBBLE STEP IS THE BEST SINGLE SANITY CHECK IN THE APPLICATION**
and `NEW-MACHINE.md` uses it as one: it exercises a square in three
layers, a tool requirement, an inventory write and the renderer in one
click.

---

## 6. The DM's side

`Active > Characters > NPCs` and `Creatures`. These two subtabs hide
themselves from anybody who is not the DM of the game, which is a UI
offer and not the boundary - **the boundary is RLS.**

| # | do | proves it | lives in |
|---|---|---|---|
| 6.1 | `NPCs`, **Write statblock** | a new NPC with a class, size, AC and hit points | `#create-npc` `create_npc()` |
| 6.2 | `Creatures`, **Import from reference**, pick a goblin | it is copied into your game as an editable creature | `#creature-import` `import_statblock()` |
| 6.3 | **Import from file** with an exported statblock | the same, from JSON | `#creature-import-file` `export_creature()` |
| 6.4 | On any character row, type a **markup** and click **shop** | it says they sell at that multiple. Blank is the off switch | `set_merchant()` |

**A BLANK MARKUP IS NOT A PRICE OF ZERO,** it is the absence of a
price, matching 040's nullable column. "Not a merchant" and "sells at
nothing" are different facts.

---

## 7. Money, and a trade

`Active > Trade`. **This does nothing until step 6.4 has made
somebody a merchant** - that is the dependency the ordering exists for.

| # | do | proves it | lives in |
|---|---|---|---|
| 7.1 | Pick your character and the merchant | both inventories list, with a wallet | `#trade-a` `wallet()` |
| 7.2 | Tick things on both sides | the balance says who owes whom and how much | `#trade-balance` `quote_trade()` |
| 7.3 | Enter Persuasion and Insight, **haggle** | the margin moves and it says what was said | `#do-haggle` `haggle_margin()` |
| 7.4 | **Trade** | the goods and coin move, both inventories redraw | `#do-trade` `execute_trade()` |
| 7.5 | **Give - no coin** instead | the goods move and no money does | `#do-give` `give_item()` |
| 7.6 | Load the pack up and read the encumbrance line | it says what carrying that much costs | `encumbrance()` |
| 7.7 | Put something in a container, then take it out | the container reports its contents and its depth cap | `list_contents()` |

---

## 8. A fight

`Active > Encounter`. The mode strip at the top is **View** and
**Edit**; Edit is the DM's.

| # | do | proves it | lives in |
|---|---|---|---|
| 8.1 | **Edit**, name an encounter, **Create** | it appears in the list, status *draft* | `#create-encounter` `create_encounter()` |
| 8.2 | **Move here** with the room selected | the encounter now happens somewhere | `#set-enc-place` `set_encounter_location()` |
| 8.3 | **Enrol** your character, then the goblin | both are actors in it, each with their own hit points | `#enrol` `enrol_actor()` |
| 8.4 | **Add** a challenge - a locked door, say | it is listed as part of the encounter | `#add-challenge` `add_challenge()` |
| 8.5 | **Begin** | initiative is rolled for everybody and the order appears | `#next-turn` `roll_initiative()` |
| 8.6 | Read the order, and the strip above the sheet | the turn order, and the player's own view of it | `#strip-panel` `fight_strip()` |
| 8.7 | On your turn, pick a target and **Roll** an attack | a card in the feed with the natural d20, the total, hit or miss | `#roll-named` `roll_named()` |
| 8.8 | Spend an action, a bonus action, a reaction | three slots, each spendable once | `spend_slot()` |
| 8.9 | **End turn** | it moves to the next actor and says whose turn it is | `#fight-next` `advance_turn()` |
| 8.10 | Hold a turn, then release it | the held action takes its named place in the order | `hold_turn()` |
| 8.11 | Attack with the goblin | the same `swing` a character's attack goes through (019) | `npc_attack()` |
| 8.12 | Drop somebody to 0 and roll a **death save** | three successes stabilise, three failures do not | `#death-save` `death_save()` |
| 8.13 | `Log` tab in the feed | everything that happened in this fight, in order | `encounter_log()` |
| 8.14 | **reset** the order | the fight starts over without being rebuilt | `#reset-order` `reset_order()` |

**THE VERDICT IS NEVER THE SCREEN'S.** Hit, miss, crit and fumble come
back from Rust as words; nothing in `main.js` compares a d20 against
20. A crit range of 19+ is a thing a weapon can have and the screen is
in no position to know.

---

## 9. Spells

You need a caster. Either make a second character with a casting class
or add one at step 2.9 - the **Spells** tab on the sheet is hidden for
anybody who casts nothing.

| # | do | proves it | lives in |
|---|---|---|---|
| 9.1 | Sheet, **Spells** | the slots by level, and what is prepared | `#prayer-slots` `list_casting()` |
| 9.2 | Find a spell and prepare it | the prepared count goes up and the slot list stands | `#prep-find` `prepare_spell()` |
| 9.3 | Write one into the book | the book lists it; a scroll is a one-page book | `list_book()` `scribe_spell()` |
| 9.4 | In a fight, read the **Cast** panel beside the roll box | the prepared list, where the rolling happens | `#cast-panel` `castable()` |
| 9.5 | Cast an attack spell at a target | a roll card, and a slot spent | `#cast-list` `cast_spell()` |
| 9.6 | Cast a **save** spell | it states the DC and waits | `resolve_spell_save()` |
| 9.7 | Resolve the save for the target | the d20 is the target's, the DC was the caster's | `resolve_spell_save()` |
| 9.8 | Upcast one a level higher | it says what the extra slot level bought | `spend_spell_slot()` |

**A SAVE SPELL IS TWO STEPS FOR THE SAME REASON AN ATTACK SPELL IS**
(158): the DC belongs to the caster and the d20 belongs to the target,
so the engine states the number and waits to be asked.

---

## 10. Time, rest and what is running

The clock bar at the top of `Active`.

| # | do | proves it | lives in |
|---|---|---|---|
| 10.1 | Read the clock | the game's own time, which only moves when told | `#clock-active` `game_clock()` |
| 10.2 | Advance it by an explicit jump | the time moves and says by how much | `#steps-active` `advance_time()` |
| 10.3 | **Short rest** | it previews what would come back, then does it | `#restsay-active` `rest_preview()` |
| 10.4 | **Long rest** | slots, hit points and uses return | `take_rest()` |
| 10.5 | Spend a limited-use feature, then rest | the use is gone, then back | `spend_use()` |
| 10.6 | Apply a condition to somebody | a band appears listing what is running, soonest to end first | `#fx-active` `apply_effect()` |
| 10.7 | End it, or let it run out | the band empties and disappears entirely | `list_effects()` `end_effect()` |
| 10.8 | Take a class feature that asks you to choose | it records the choice and what it owes you | `choose_feature()` |
| 10.9 | Perform for an audience | karma and the audience, which is 076 | `#do-perform` `perform()` |

**EXPLICIT JUMPS, AND NO GUESSING.** Outside a fight nothing can know
how long anything took, and an engine that guessed would be
confidently wrong. The band is gone entirely when nothing is running,
rather than saying "no effects" on every session that never uses one.

---

## 11. The feed, and the die

The right-hand column, open by default.

| # | do | proves it | lives in |
|---|---|---|---|
| 11.1 | `Rolls` tab | every roll, newest first, with who rolled and what for | `#rolls` `list_rolls()` |
| 11.2 | Roll some loose dice | a card with the formula and the result | `#roll-formula` `roll_dice()` |
| 11.3 | **Insert roll** by hand | a card the DM wrote rather than rolled | `#create-roll` `create_roll()` |
| 11.4 | **Patch narrative** on a card | the prose changes and the numbers do not | `#patch-roll` `patch_roll_narrative()` |
| 11.5 | Look at the dice pad after any d20 | a three-dimensional d20 turned to the face you rolled, and the line naming the roll | `#d20` `#die-say` |
| 11.6 | Change **die** to red | it turns red at once, and stays red next session on any machine | `#die-colour` `set_dice_prefs()` |
| 11.7 | Change **numbers** to white | the glyphs change, keeping a stroke of the opposite tone | `#pip-colour` `dice_prefs()` |
| 11.8 | `Log` tab | the developer log. **clear** and **hide** are there | `#clear-log` `#hide-log` |

**THE DIE SHOWS A NUMBER IT DID NOT CHOOSE** (290). The face comes
from `natural_roll` on the row, which came from Rust; the pad draws a
verdict it is told. Green and red are `face_outcome` and not a
comparison.

**THE COLOUR IS A FACT ABOUT YOU AND NOT ABOUT THIS LAPTOP** (292), so
it is a table keyed on `current_profile()` rather than `localStorage` -
unlike whether the feed is open and which tab it is showing, which are
facts about the machine and are stored there on purpose.

---

## What this tour does not reach

Said here so you do not go looking.

- **Advantage shows one die.** The pad draws the one face the row
  carries; `RollResult` has no `faces` list yet.
- **The dice pad is a display and not a control.** No button on it
  rolls anything.
- **Only the d20 is drawn in three dimensions.** The damage dice are
  the same code with a different net, except the d10, which is a
  trapezohedron rather than a Platonic solid.
- **An ability check has no button.** The engine takes one; the screen
  does not ask for one.
- **`#mv-who` offers no creatures,** so a monster cannot be placed on
  the map from there.
- **44 bestiary statblock keys are humanoid,** so goblins, kobolds,
  orcs and lycanthropes land in `NPCs` rather than `Creatures`. 140's
  rule working exactly as written, never examined from the monster end.
- **Not built at all:** Journal, `Active > Map` as its own pane, the
  proposal and mod queue, `export_character`, `Campaign > Encounters`,
  DM-edit attribution, a gallery for character pictures, and anything
  that resizes or crops an upload.
- **Nothing in the frontend has a test** beyond `tests/map.smoke.mjs`
  and this file's own name check.

---

## When a step is wrong

Fix this file. It is checked mechanically for names and by nobody at
all for sense, which means the first walk-through is worth more than
the next hundred. **This file is a draft forever and is meant to be
edited** - which is the opposite of a migration, where 115's rule says
an applied file is a record and corrections go in a new one.
