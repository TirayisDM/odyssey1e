-- =====================================================================
-- 024_name_actor_after_convergence.sql
-- odyssey1e — 022 unreachable-branched 018's naming
-- =====================================================================
--
-- THE REGRESSION. 018 named an unnamed actor `<Species> <Class> 0001`.
-- It decided what kind of thing it was naming like this:
--
--     if new.character_id is not null then
--       -- one Rodnar; take the character's name, no number
--
-- which was correct when only PLAYER characters had a character_id.
-- 022 made character_id NOT NULL on every actor, so that branch now
-- catches everything and the ordinal naming became unreachable. Enrol
-- three goblins and all three are called "Goblin".
--
-- Worse than a cosmetic collision: a roll takes its name from the
-- SHEET, and the sheet is now the character. Three goblins sharing one
-- name means a roll log that cannot say which one swung - the exact
-- defect 021 fixed, arriving again by a different road.
--
-- Caught by enrolling one through the live function and reading the
-- label back before shipping, which is the check 421d08c and fbffc40
-- were committed without.
--
-- THE FIX. Discriminate on npc_key, which is what the question was
-- always really about: an individual made from a TYPE gets numbered
-- within its type, and a person gets their own name. 022 kept npc_key
-- precisely as this kind of provenance.
--
-- AND THE TWO NAMES ARE KEPT EQUAL. The actor's label and the
-- character's name are the same fact seen from two places - what the
-- table calls this creature. The label is computed here, so the
-- character is updated to match in the same breath. Letting them drift
-- would mean the roster said "Goblin 0003" while every roll it made
-- said "Goblin".
-- =====================================================================

create or replace function public.name_actor()
returns trigger
language plpgsql
security invoker
set search_path = ''
as $$
declare
  base text;
  n    integer;
  gid  uuid;
begin
  -- An explicit name wins and is left exactly as given. instantiate_npc
  -- already gave the character the same one.
  if new.label is not null and btrim(new.label) <> '' then
    return new;
  end if;

  -- A PERSON, not made from a type. One Rodnar; a number would be absurd.
  if new.npc_key is null then
    select c.name into base
      from public.characters c where c.id = new.character_id;
    new.label := coalesce(nullif(btrim(base), ''), 'Adventurer');
    return new;
  end if;

  -- Species, class, both or neither - and the type's own name when it
  -- carries none of them. concat_ws skips NULLs, so the three-way
  -- choice needs no branching.
  select nullif(btrim(concat_ws(' ', nullif(btrim(n2.species), ''),
                                     nullif(btrim(n2.class), ''))), ''),
         e.game_id
    into base, gid
    from public.encounters e
    left join public.npcs n2
      on n2.key = new.npc_key
     and (n2.game_id = e.game_id or n2.game_id is null)
   where e.id = new.encounter_id
   order by n2.game_id nulls last
   limit 1;

  if base is null then
    select nullif(btrim(n2.name), '') into base
      from public.npcs n2
     where n2.key = new.npc_key
     order by n2.game_id nulls last
     limit 1;
  end if;

  base := coalesce(base, 'Creature');

  -- Per GAME, not per encounter. See 018.
  select coalesce(max(ea.name_ordinal), 0) + 1 into n
    from public.encounter_actors ea
    join public.encounters e2 on e2.id = ea.encounter_id
   where e2.game_id = gid
     and ea.name_base = base;

  new.name_base    := base;
  new.name_ordinal := n;
  new.label        := base || ' ' || lpad(n::text, 4, '0');

  -- The individual is called what the table calls it. One fact.
  update public.characters
     set name = new.label
   where id = new.character_id;

  return new;
end;
$$;

comment on function public.name_actor() is
  'Names an actor enrolled without a label: a person takes their own name, an individual made from a type takes <Species> <Class> 0001 numbered per game. Discriminates on npc_key since 024 - 022 made character_id NOT NULL on every actor, which made the old test always true. Keeps the character''s name equal to the label, because they are one fact.';

revoke all on function public.name_actor() from public, anon, authenticated;
