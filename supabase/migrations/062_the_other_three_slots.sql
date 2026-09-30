-- =====================================================================
-- 062_the_other_three_slots.sql
-- odyssey1e — a bonus action, a reaction and a free interaction
-- =====================================================================
--
-- 061 MADE `bonus`, `reaction` AND `free` LEGAL AND LEFT THEM EMPTY,
-- and said so: "the column admits them so the first one is a write
-- rather than a migration, and the screen does NOT show counters for
-- slots nothing can fill - an always-zero bonus 0/1 is a claim that a
-- system exists."
--
-- This is that write. Nothing about the shape changes; what changes is
-- that something now produces the values.
--
-- ---------------------------------------------------------------------
-- A SLOT IS SPENT BY WRITING AN ACTION
-- ---------------------------------------------------------------------
--
-- Not by a per-turn state table, which was the alternative and would
-- have been a second account of the same round. An action row already
-- gets its round stamped (054, 060), already records whose turn it was
-- taken in, already appears in the log, and already deletes cleanly -
-- which is what un-ticking a box has to do.
--
-- So "Garn used his bonus action" is an action with no dice. That is a
-- shape the table already permits: `key` defaults, `status` defaults to
-- resolved, and every roll column lives on `rolls` rather than here.
-- The log gains a line saying a slot was spent, which is history worth
-- having rather than a blank.
--
-- ---------------------------------------------------------------------
-- THE KEY AND THE COST AGREE BY CONSTRUCTION
-- ---------------------------------------------------------------------
--
-- 061's trigger read `attack` from the key and called everything else
-- an action. A marker written with `key = 'bonus'` would therefore have
-- been stamped `cost = 'action'` unless the caller also passed the
-- cost - two things to get right, and the kind of pair that drifts.
--
-- The trigger now recognises a cost word used as a key. A caller may
-- still state a cost and it is still kept, which is what an attack
-- technique that one day costs a bonus action will need.
--
-- ---------------------------------------------------------------------
-- WHAT THIS DOES NOT DECIDE
-- ---------------------------------------------------------------------
--
-- A REACTION IS ONCE PER ROUND AND THE OTHERS ARE ONCE PER TURN. 5e
-- refreshes a reaction at the start of your turn, so a creature that
-- acts once per round is the same either way - which every creature
-- here does. The counting is per ROUND (054), it is right for every
-- case the app can currently produce, and it will need revisiting the
-- day something takes two turns in one round. Said here rather than
-- discovered then.
--
-- NOTHING IS REFUSED. 051 decided the order informs and never refuses;
-- a slot is the same. Ticking a box a second time is possible and reads
-- as over budget, because a DM granting a second bonus action is an
-- ordinary Tuesday and an app that argues about it is one they fight.
--
-- A HELD ATTACK still has no home. Readying is an action that BECOMES a
-- reaction on a trigger, so it needs both halves and something to fire
-- it. What 062 gives it is the reaction slot to land in.
-- =====================================================================

create or replace function public.stamp_action_cost()
returns trigger
language plpgsql
security definer
set search_path = public
as $$
begin
  if new.cost is null then
    new.cost := case
      when new.key = 'attack' then 'attack'
      -- A COST WORD USED AS A KEY IS THAT COST. 062's markers are
      -- written this way, and it means the caller states one thing
      -- rather than two that can disagree.
      when new.key in ('bonus', 'reaction', 'free', 'action') then new.key
      else 'action'
    end;
  end if;
  return new;
end $$;

-- The trigger itself is unchanged - only the function it calls - but
-- it is recreated so a database that somehow lost it comes back whole.
drop trigger if exists actions_stamp_cost on public.actions;
create trigger actions_stamp_cost
  before insert on public.actions
  for each row execute function public.stamp_action_cost();

comment on column public.actions.cost is
  'What this action spent: attack, action, bonus, reaction or free. Derived from `key` by trigger unless the caller states one - a swing costs an attack, a cost word used as a key IS that cost, and everything else costs the action. 062 writes the last three from the slot ticks on the fight card.';
