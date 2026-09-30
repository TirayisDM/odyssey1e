-- =====================================================================
-- 061_how_many_swings_do_you_get.sql
-- odyssey1e — Extra Attack, and what an action cost
-- =====================================================================
--
-- 054 COUNTED ACTIONS AND CALLED A SECOND ONE "BEYOND ONE TURN". It
-- said so honestly at the time: "Extra Attack, haste, action surge and
-- a legendary action all make this true and legitimate, and the engine
-- knows about none of them."
--
-- It knows about one of them now, and it is the common one. Garn is a
-- level 5 Barbarian. He gets TWO attacks. Every second swing he has
-- ever taken has been flagged as irregular by an app that had no way to
-- know it was owed to him - and a warning that fires on correct play is
-- worse than no warning, because a DM learns to ignore it and then
-- misses the one that mattered.
--
-- ---------------------------------------------------------------------
-- THE PROGRESSION IS DATA, NOT A MATCH ARM
-- ---------------------------------------------------------------------
--
-- `extra_attack_levels` is the list of levels at which a class gains
-- ANOTHER attack, so the count is `1 + how many of them you have
-- reached`. A Fighter is {5,11,20} and therefore 1, 2, 3, 4 across
-- twenty levels; a Barbarian is {5} and therefore 1 then 2.
--
-- ON THE CLASS ROW because 055 put classes in a catalogue with nullable
-- tenancy precisely so a table could write its own. A DM who wants a
-- Fighter that reaches four attacks at 17 edits a row. A match arm in
-- Rust would make that a code change, and this campaign is already all
-- custom species - the classes will follow.
--
-- EMPTY IS THE COMMON CASE. Seven of the twelve never gain one, and an
-- empty array says that without a NULL meaning "unknown". The Bard's
-- Extra Attack comes from a SUBCLASS at 6, and 055 deliberately has no
-- subclasses, so the Bard is empty and will be wrong for a College of
-- Swords bard until subclasses exist. Written down here rather than
-- discovered later.
--
-- ---------------------------------------------------------------------
-- WHAT AN ACTION COST
-- ---------------------------------------------------------------------
--
-- 054 refused to classify 193 techniques into an action economy,
-- because inventing the answer would have put 193 guesses in the
-- database. That argument was right and it does not apply here, for
-- two reasons.
--
-- FIRST, THIS IS DERIVED, NOT GUESSED. `actions.key` is already the
-- engine's vocabulary - `attack`, `death`, or a skill key - and the
-- cost follows from it with nothing invented: a swing costs an attack,
-- everything else costs the action. The trigger reads the column that
-- is already there.
--
-- SECOND, THE TECHNIQUES ARE ALL ONE THING. Checked rather than
-- assumed: every one of the 193 rows has dice and a weapon mode -
-- melee, ranged or thrown. They are weapon attack techniques. There is
-- no bonus-action technique in the catalogue to misclassify, so the
-- classification 054 refused is not the classification being made.
--
-- WHAT IS DELIBERATELY STILL EMPTY: `bonus`, `reaction` and `free` are
-- legal values and NOTHING WRITES THEM. There is no bonus-action path,
-- no readied action, and no reaction in the app. The column admits them
-- so the day one arrives is a write rather than a migration, and the
-- screen does NOT show counters for slots nothing can fill - an
-- always-zero "bonus 0/1" is a claim that a system exists.
--
-- A HELD ATTACK is the one Dave named that has no home yet. Readying is
-- an action that converts into a reaction on a trigger, so it needs
-- both halves and a trigger to hang on. `reaction` is here for it.
-- =====================================================================

-- ---------------------------------------------------------------------
-- HOW MANY ATTACKS
-- ---------------------------------------------------------------------

alter table public.classes
  add column if not exists extra_attack_levels integer[] not null default '{}';

comment on column public.classes.extra_attack_levels is
  'Levels at which this class gains ANOTHER attack in the Attack action. Attacks = 1 + how many of these the character has reached. Fighter {5,11,20}; Barbarian, Paladin, Ranger, Monk {5}; everyone else empty. On the row rather than in code so a table can write its own class - 055''s whole point.';

alter table public.classes
  drop constraint if exists classes_extra_attacks_are_levels;
alter table public.classes
  -- A literal rather than generate_series: a check constraint cannot
  -- contain a subquery, which is the kind of thing you find out by
  -- trying it.
  add constraint classes_extra_attacks_are_levels check (
    extra_attack_levels <@
      '{1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20}'::integer[]
  );

update public.classes set extra_attack_levels = '{5,11,20}'
 where key = 'fighter' and game_id is null;
update public.classes set extra_attack_levels = '{5}'
 where key in ('barbarian', 'paladin', 'ranger', 'monk') and game_id is null;

-- ---------------------------------------------------------------------
-- WHAT IT COST
-- ---------------------------------------------------------------------

alter table public.actions
  add column if not exists cost text;

comment on column public.actions.cost is
  'What this action spent: attack, action, bonus, reaction or free. DERIVED from `key` by trigger, not guessed - a swing costs an attack and everything else costs the action. bonus/reaction/free are legal and nothing writes them yet; the column admits them so the first one is a write rather than a migration.';

alter table public.actions
  drop constraint if exists actions_cost_is_a_cost;
alter table public.actions
  add constraint actions_cost_is_a_cost check (
    cost is null or cost in ('attack', 'action', 'bonus', 'reaction', 'free')
  );

-- A SWING COSTS AN ATTACK, EVERYTHING ELSE COSTS THE ACTION.
--
-- A death save is the turn of a creature that is dying, so it costs the
-- action - 015's decision, restated as a cost rather than re-argued.
create or replace function public.stamp_action_cost()
returns trigger
language plpgsql
security definer
set search_path = public
as $$
begin
  -- A cost the caller states is kept. Nothing states one today, and
  -- the day a bonus action exists it will.
  if new.cost is null then
    new.cost := case when new.key = 'attack' then 'attack' else 'action' end;
  end if;
  return new;
end $$;

drop trigger if exists actions_stamp_cost on public.actions;
create trigger actions_stamp_cost
  before insert on public.actions
  for each row execute function public.stamp_action_cost();

-- Every action already written, on the same rule. 77 rows, and the
-- derivation is the same one the trigger applies - so history and
-- future agree rather than the backfill being a second opinion.
update public.actions
   set cost = case when key = 'attack' then 'attack' else 'action' end
 where cost is null;
