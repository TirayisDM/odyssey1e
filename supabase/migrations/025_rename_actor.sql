-- =====================================================================
-- 025_rename_actor.sql
-- odyssey1e — giving a goblin a name it earned
-- =====================================================================
--
-- "Goblin 0003" is what an unnamed thing is called until it does
-- something. A DM who wants Snaggletooth should be able to say so.
--
-- WHY THIS IS A FUNCTION AND NOT TWO UPDATES. 024 settled that the
-- actor's label and the character's name are ONE FACT seen from two
-- places - what the table calls this creature. The roster reads the
-- label; every roll takes its name from the sheet, which is the
-- character. Writing them separately means a window where the roster
-- says Snaggletooth and the roll log says Goblin 0003, and a failure
-- halfway leaves that permanently.
--
-- THE ORDINAL IS NOT RELEASED. name_base and name_ordinal stay exactly
-- as they were, and that is deliberate: the next auto-named goblin is
-- max(ordinal)+1, so clearing them would free 0003 to be issued again.
-- A number that has been in a roll log is spent - the same reasoning
-- 018 used to choose max() over count(). Snaggletooth remains, in the
-- bookkeeping, the third goblin this campaign made.
--
-- PAST ROLLS KEEP THE OLD NAME, and nothing here tries to change that.
-- They snapshotted it, freeze_roll_identity makes those snapshots
-- immutable, and "Goblin 0003 hit Rodnar" remains a true statement
-- about what was recorded at the time. The log is evidence, not an
-- index that should be kept tidy.
--
-- DM ONLY, and not because this says so. The actor_update_scope trigger
-- from 016 refuses a label change from anyone who is not the DM of the
-- game, and characters carries its own owner-or-DM policy. This is
-- SECURITY INVOKER, so both apply to whoever calls it.
-- =====================================================================

create function public.rename_actor(
  p_actor_id uuid,
  p_name     text
)
returns text
language plpgsql
security invoker
set search_path = ''
as $$
declare
  cid   uuid;
  clean text;
begin
  clean := nullif(btrim(p_name), '');
  if clean is null then
    -- Blank means "no name" only at ENROLMENT, where 018 fills it in.
    -- Renaming to nothing would leave a creature the roster cannot
    -- print, so it is refused rather than reinterpreted.
    raise exception 'a name cannot be blank - delete the actor instead';
  end if;

  select character_id into cid
    from public.encounter_actors
   where id = p_actor_id;

  if cid is null then
    raise exception 'no such actor, or it is not visible to you';
  end if;

  update public.encounter_actors set label = clean where id = p_actor_id;
  update public.characters        set name  = clean where id = cid;

  return clean;
end;
$$;

comment on function public.rename_actor(uuid, text) is
  'Rename a creature. Moves the actor''s label and the character''s name together, because 024 made them one fact - the roster reads one and every roll takes its name from the other. Leaves name_base and name_ordinal alone: a spent ordinal is not reissued. Past rolls keep the name they snapshotted.';

revoke all on function public.rename_actor(uuid, text) from public, anon;
grant execute on function public.rename_actor(uuid, text) to authenticated;
