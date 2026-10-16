-- 208. AND YOUR OWN INITIATIVE, ONCE.
--
-- 205 put a "Roll initiative" button on a player's own line in the
-- fight strip. Pressing it said:
--
--   only the DM may change an actor beyond its death saves
--
-- `guard_actor_update` is a BEFORE UPDATE trigger on `encounter_actors`
-- and it is the real rule - not the RLS policy. A non-DM may change the
-- death-save columns and nothing else, `initiative` included.
--
-- ---------------------------------------------------------------------
-- AND THE ENTRY UNDER 204 THAT SAID OTHERWISE WAS WRONG
-- ---------------------------------------------------------------------
--
-- That note read: "any player can write any column on any actor row:
-- initiative, hp_override, dead, even the label". It was arrived at by
-- reading `pg_policy` and stopping there. **RLS was never the whole
-- answer for this table** - a policy cannot restrict a column, so 016
-- put the column rule in a trigger, which is exactly the right place
-- and exactly where nobody looked.
--
-- The honest version: a non-DM member may change `death_successes`,
-- `death_failures`, `dead`, the two hold columns and the naming
-- columns. That is wider than the error message claims - `dead` on
-- somebody else's monster is not a death save - but it is nothing like
-- "any column".
--
-- ---------------------------------------------------------------------
-- WHAT THIS OPENS, AND HOW FAR
-- ---------------------------------------------------------------------
--
-- Exactly what `reveal::needs_initiative` already offers, which is the
-- point of the two agreeing:
--
--   YOURS      the actor must be a character you own. Rolling for
--              somebody else is reordering the fight, which is the DM's.
--   ONCE       only while it is unrolled. A re-roll mid-fight is the
--              same act under a friendlier name, so it stays refused
--              and the button stops being offered after the first.
--
-- NOT SECURITY DEFINER, deliberately - this trigger never was. It reads
-- `characters` as the caller, so RLS applies to the ownership test and
-- a row the caller cannot see answers "not mine", which fails closed.
-- `current_profile()` is the 197 resolver, so this is the PERSON and
-- not the sign-in.

create or replace function public.guard_actor_update()
returns trigger
language plpgsql
set search_path to ''
as $fn$
declare
  dm   boolean;
  mine boolean;
begin
  select public.is_game_dm(e.game_id) into dm
    from public.encounters e
   where e.id = new.encounter_id;

  if coalesce(dm, false) then
    return new;
  end if;

  -- EVERYTHING THE DM OWNS, unchanged from before except that
  -- `initiative` has come out of this list and is judged below.
  if (new.encounter_id, new.character_id, new.npc_key, new.label,
      new.ac_override, new.hp_override, new.active,
      new.enrolled_at)
     is distinct from
     (old.encounter_id, old.character_id, old.npc_key, old.label,
      old.ac_override, old.hp_override, old.active,
      old.enrolled_at)
  then
    raise exception
      'only the DM may change an actor beyond its death saves';
  end if;

  -- 208. YOUR OWN, ONCE.
  if new.initiative is distinct from old.initiative then
    select exists (
      select 1
        from public.characters c
       where c.id = new.character_id
         and c.owner_uid = public.current_profile()
    ) into mine;

    if not coalesce(mine, false) then
      raise exception
        'only the DM may roll initiative for somebody else';
    end if;

    if old.initiative is not null then
      raise exception
        'your initiative is already rolled - ask the DM to change it';
    end if;
  end if;

  return new;
end;
$fn$;

-- 195's check, since this replaces a function.
do $gate$
declare n int;
begin
  select count(*) into n from public.security_doors();
  if n > 0 then
    raise exception '208: the wall has % hole(s)', n;
  end if;
end
$gate$;
