-- 149. WHAT A CREATURE MAY DO OUT OF TURN.
--
-- 148's header said legendary actions were the one of the four missing
-- things that FITS, and that most of the work was already done. It was:
--
--   054  stamps the round onto every action
--   060  stamps whose turn it was
--   spent::out_of_turn  has compared the two since 054
--   spent.rs  counts what a creature has spent in a round
--
-- Everything needed to say "the dragon has used two of its three" was
-- there except the three. This is the three.
--
-- ---------------------------------------------------------------------
-- A COLUMN, BECAUSE THERE IS NO RULE TO DERIVE IT FROM
-- ---------------------------------------------------------------------
--
-- Every other budget in `spent::Budget` is worked out: attacks come from
-- the class's Extra Attack progression, and the action, bonus action,
-- reaction and free interaction are one each because 5e says so. A
-- legendary allowance is none of that - it is a number printed on a
-- statblock, three for everything that has one and absent for everything
-- else. So it is stored, which is what 001 means by "facts live in
-- Postgres".
--
-- THIRTY CREATURES, READ OFF THE PUBLISHED SRD like 146, 147 and 148:
-- the twenty adult and ancient dragons, both sphinxes, the Aboleth,
-- Kraken, Lich, Mummy Lord, Solar, Tarrasque, Vampire and the Unicorn.
-- All thirty have three. Young dragons and wyrmlings have none, which is
-- why the column is seeded by name rather than by creature type.
--
-- The Dragon Turtle has none either, which surprised me enough to check
-- twice.
--
-- ---------------------------------------------------------------------
-- WHAT IT CHANGED IN THE COUNTING, WHICH IS MORE THAN IT SOUNDS
-- ---------------------------------------------------------------------
--
-- `this_round` now puts an out-of-turn action in its own tally instead
-- of against the creature's ordinary budget. That is a correctness fix
-- that happens to arrive with this feature: an action taken on somebody
-- ELSE'S turn does not spend this creature's turn, and counting it
-- there made a dragon doing exactly what the book says - three
-- legendary actions, then its own turn - read as four actions and light
-- up as over budget.
--
-- 061 already had to fix a warning that fired on correct play once, for
-- Extra Attack, and wrote down why: "a warning that fires on correct
-- play is worse than no warning, because a DM learns to ignore it and
-- then misses the one that mattered."
--
-- ZERO IS THE INTERESTING VALUE. Almost nothing has legendary actions,
-- so almost any out-of-turn action is now flagged - a goblin swinging
-- on the wizard's turn is precisely what a DM wants to see, and it is
-- what 054's `beyond_one_turn` was reaching for before it could tell
-- that case apart from Extra Attack.
--
-- A REACTION IS ALSO OUT OF TURN and keeps its own slot, because `cost`
-- can name it. It is the only out-of-turn thing this engine can tell
-- apart, and an opportunity attack reading as a legendary action would
-- be the usual costume on the usual fault.
--
-- ---------------------------------------------------------------------
-- WHAT THIS IS STILL NOT
-- ---------------------------------------------------------------------
--
-- It does not REFUSE anything, which is 051's decision about turn order
-- and 061's about budgets, held to. It says what is owed and what has
-- been spent; a DM who wants a fourth takes a fourth.
--
-- It does not know a wing attack costs TWO. 5e prices some legendary
-- options at two of the three, and no technique in this catalogue says
-- what it costs - 054 refused to guess that for 193 rows and the
-- refusal still holds. Every legendary action counts as one here, and
-- the day `techniques` carries a cost this reads it instead.
--
-- The RESET is per round rather than at the start of the creature's own
-- turn, which is where 5e puts it. The two differ only for an action
-- taken between the top of the round and that creature's initiative,
-- and the round is the only window `actions.round` can express.

alter table public.npcs
  add column if not exists legendary_actions integer not null default 0;

alter table public.npcs
  drop constraint if exists npcs_legendary_actions_check;
alter table public.npcs
  add constraint npcs_legendary_actions_check
  check (legendary_actions >= 0 and legendary_actions <= 5);

comment on column public.npcs.legendary_actions is
  'How many actions this creature may take OUT OF ITS OWN TURN each round (149). Three for the thirty SRD creatures that have them, zero for everything else - and zero is the interesting value, because it makes any out-of-turn action by an ordinary creature show up. Stored rather than derived: unlike every other budget in spent::Budget there is no rule behind it, only a number printed on a statblock. Nothing refuses a fourth; the count informs, per 051 and 061.';

update public.npcs
   set legendary_actions = 3
 where game_id is null
   and key in (
     'adult_black_dragon','adult_blue_dragon','adult_brass_dragon',
     'adult_bronze_dragon','adult_copper_dragon','adult_gold_dragon',
     'adult_green_dragon','adult_red_dragon','adult_silver_dragon',
     'adult_white_dragon',
     'ancient_black_dragon','ancient_blue_dragon','ancient_brass_dragon',
     'ancient_bronze_dragon','ancient_copper_dragon','ancient_gold_dragon',
     'ancient_green_dragon','ancient_red_dragon','ancient_silver_dragon',
     'ancient_white_dragon',
     'aboleth','androsphinx','gynosphinx','kraken','lich',
     'mummy_lord','solar','tarrasque','unicorn','vampire'
   );
