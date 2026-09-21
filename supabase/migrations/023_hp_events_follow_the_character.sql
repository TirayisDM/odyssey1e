-- =====================================================================
-- 023_hp_events_follow_the_character.sql
-- odyssey1e — damage lands on the individual, now that there is one
-- =====================================================================
--
-- 013 gave hp_events an XOR: a character OR an actor, never both,
-- because a player's hit points belong to the player and follow them
-- between fights, while an NPC instance had no character to belong to
-- and its damage had to hang off the actor row.
--
-- 022 removed the second half of that sentence. Every participant is a
-- character now, so every wound has a character to land on, and the
-- either/or is a distinction with nothing left on one side of it.
--
-- IT ALSO HAD TO BE FIXED IMMEDIATELY, not tidied up later. hp_current
-- is derived as hp_max plus the sum of the damage aimed at a subject.
-- After 022 the engine looks that up by character, and eleven events
-- worth fifty-four points were still pointing at actor rows - Goblin 1,
-- Goblin 2 and Goblin Fighter 0001 would have stood up at full health
-- with their wounds still in the log but attached to nothing anyone
-- reads. A silent full heal is exactly the class of bug 022 exists to
-- prevent, so leaving it for a later migration was not an option.
--
-- actor_id SURVIVES AS PROVENANCE, the same way npc_key did in 022.
-- Which appearance a wound was taken in is a real fact and worth
-- keeping; it is simply not the answer to "whose hit points are these".
-- =====================================================================

-- THE XOR GOES FIRST, for the reason 022 learned the same way: it reads
-- "one subject or the other, never both", so it refuses the very update
-- meant to satisfy it. A row mid-repoint legitimately has both.
alter table public.hp_events drop constraint hp_events_one_subject_check;

-- Every wound recorded against an actor now names the individual that
-- actor is. Same creature, said properly.
update public.hp_events h
   set character_id = ea.character_id
  from public.encounter_actors ea
 where h.actor_id = ea.id
   and h.character_id is null;

-- Nothing may be left without a subject: an event that cannot say whose
-- hit points it moved is a number nobody can apply.
do $$
declare orphans integer;
begin
  select count(*) into orphans from public.hp_events where character_id is null;
  if orphans > 0 then
    raise exception '023: % hp_event(s) have no character to land on', orphans;
  end if;
end $$;

alter table public.hp_events alter column character_id set not null;

comment on column public.hp_events.character_id is
  'WHOSE HIT POINTS THESE ARE. NOT NULL since 023: every participant is a character, including monsters, so every wound has an individual to land on. Two goblins off one statblock still bleed separately, because 022 made them two characters.';
comment on column public.hp_events.actor_id is
  'PROVENANCE ONLY since 023. Which appearance in which encounter the wound was taken in - a real fact, and worth keeping, but not the answer to whose hit points moved. NULL for damage recorded outside an encounter.';
