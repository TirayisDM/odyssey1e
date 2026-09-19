-- =====================================================================
-- 018_npc_naming.sql
-- odyssey1e — an unnamed NPC names itself
-- =====================================================================
--
-- WHY THIS EXISTS: enrolling an actor meant inventing a label by hand,
-- because encounter_actors.label was NOT NULL. That is fine for the two
-- goblins somebody typed in SQL and hopeless for a DM screen adding six
-- of them mid-fight. An NPC should be able to have a name, and should
-- not have to.
--
-- It also closes the same hole 421d08c closed for death saves, one step
-- earlier. A roll snapshots WHOSE action it was; if nothing gives the
-- instance a name, the log ends up saying 'Someone'. Naming at
-- enrollment means every roll after it has a real name to snapshot.
--
-- THE NAME
--
--   <Species> <Class> 0001
--
-- Species and class both optional, joined by a space when both exist,
-- then a four-digit ordinal. Everything degrades without a special case:
--
--   species Goblin, no class      -> Goblin 0001
--   species Goblin, class Wizard  -> Goblin Wizard 0001
--   no species, class Cultist     -> Cultist 0001
--   neither                       -> the statblock's own name, Goblin 0001
--
-- CLASS IS NOT REQUIRED and neither is species. Most NPCs are a species
-- and nothing else; a few are a role with no species worth naming. Both
-- columns are nullable and neither has a default, because an empty
-- string and "nobody has said" are different facts.
--
-- THE ORDINAL IS PER GAME, NOT PER ENCOUNTER. Four digits is the tell:
-- nobody fields 9,999 goblins in one fight, so the number is not there
-- to separate this fight's goblins from each other - it is there to
-- separate every goblin the campaign has ever spawned. Goblin 0007 means
-- one specific goblin forever, and a roll log read three sessions later
-- still knows which one it was. Per-encounter numbering would restart at
-- 0001 every fight and make the log ambiguous the moment it matters.
--
-- WHY name_base AND name_ordinal ARE STORED. The label alone would force
-- the next ordinal to be recovered by regex, which is a parser nobody
-- asked for and one rename away from breaking. Storing the two parts
-- makes the next number a max() and makes it obvious which rows were
-- auto-named: an explicitly named actor leaves both NULL.
--
-- max(), NOT count(). Deleting Goblin 0002 must not cause the next
-- goblin to be named Goblin 0002 as well - a number that has been in a
-- roll log is spent, and reissuing it would make two different creatures
-- share a name in the record.
--
-- A CHARACTER ACTOR takes the character's name when no label is given,
-- with no ordinal. There is only one Rodnar and numbering him would be
-- absurd.
--
-- WHAT THIS DOES NOT DO: it does not rename the actors already enrolled.
-- Goblin 1 and Goblin 2 were named by hand, they are in the roll log
-- under those names, and the record principle says history stays as it
-- was recorded. They keep NULL name_base, which is also the honest
-- answer - nothing auto-named them.
-- =====================================================================

-- ---------------------------------------------------------------------
-- The two optional facts a name can be built from.
-- ---------------------------------------------------------------------

alter table public.npcs add column species text;
alter table public.npcs add column class   text;

comment on column public.npcs.species is
  'What it is: Goblin, Ogre, Wolf. Optional. Used to build a name for an instance enrolled without one. NULL means nobody has said, which is not the same as an empty string.';
comment on column public.npcs.class is
  'What it does, when that is the distinguishing fact: Wizard, Cultist, Bandit Captain. OPTIONAL AND DELIBERATELY NOT REQUIRED - most NPCs are a species and nothing more. Appended after species when both exist.';

-- The one statblock that exists. Its name was already its species.
update public.npcs set species = 'Goblin' where key = 'goblin' and game_id is null;

-- ---------------------------------------------------------------------
-- The label carries its parts when it was derived.
--
-- NOT NULL STAYS. A BEFORE ROW trigger runs before constraints are
-- checked, so passing NULL is how a caller says "no name given" and the
-- constraint still guarantees no row ever lands without one. Dropping it
-- and re-adding a CHECK would have been the same rule, stated twice,
-- with a window in between. Verified before relying on it.
-- ---------------------------------------------------------------------

alter table public.encounter_actors add column name_base    text;
alter table public.encounter_actors add column name_ordinal integer
  check (name_ordinal is null or name_ordinal >= 1);

comment on column public.encounter_actors.label is
  'What the players see and what a roll snapshots. Still NOT NULL, but PASS NULL TO MEAN "no name given" - the 018 trigger fills it before the constraint is checked, so the column is never null on a row that landed. Supply a name to keep it; the trigger leaves it alone.';
comment on column public.encounter_actors.name_base is
  'The name without its number, for an auto-named actor: "Goblin", "Goblin Wizard". NULL means this actor was named by hand. Stored rather than parsed back out of label, which would be a regex one rename away from breaking.';
comment on column public.encounter_actors.name_ordinal is
  'The number in the label, per game per name_base. Taken from max()+1, never count()+1: a number that has appeared in a roll log is spent, and reissuing it would put two creatures in the record under one name.';

-- ---------------------------------------------------------------------
-- The trigger that does the naming.
-- ---------------------------------------------------------------------

create function public.name_actor()
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
  -- An explicit name wins and is left exactly as given.
  if new.label is not null and btrim(new.label) <> '' then
    return new;
  end if;

  if new.character_id is not null then
    -- One Rodnar. A number would be absurd.
    select c.name into base
      from public.characters c where c.id = new.character_id;
    new.label := coalesce(nullif(btrim(base), ''), 'Adventurer');
    return new;
  end if;

  -- Species, class, both or neither - and the statblock's own name when
  -- it carries none of them. concat_ws skips NULLs, so the three-way
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

  -- Per GAME, not per encounter. See the header.
  select coalesce(max(ea.name_ordinal), 0) + 1 into n
    from public.encounter_actors ea
    join public.encounters e2 on e2.id = ea.encounter_id
   where e2.game_id = gid
     and ea.name_base = base;

  new.name_base    := base;
  new.name_ordinal := n;
  new.label        := base || ' ' || lpad(n::text, 4, '0');
  return new;
end;
$$;

comment on function public.name_actor() is
  'Names an actor enrolled without a label: the character''s name, or <Species> <Class> 0001 built from the statblock. The ordinal is per game so one number means one creature for the life of the campaign. Does nothing when a label was supplied.';

create trigger encounter_actors_name
  before insert on public.encounter_actors
  for each row execute function public.name_actor();

-- 017's pattern, applied on the way in rather than a migration later:
-- search_path pinned above, and a trigger function is callable by
-- nobody. Trigger bodies are exempt from the EXECUTE check that makes
-- this dangerous for a column DEFAULT - see 003.
revoke all on function public.name_actor() from public, anon, authenticated;
