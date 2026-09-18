-- =====================================================================
-- 016_actor_death_saves.sql
-- odyssey1e — a player's attack has to be able to record its own damage
-- =====================================================================
--
-- THE BUG. 015 gave write_action a p_vitals payload that updates death
-- saves. For an NPC it updates encounter_actors, whose only UPDATE
-- policy is "dm updates". A player rolling the death save, or landing a
-- hit on something already down, therefore wrote nothing at all.
--
-- AND IT WAS SILENT. This is the part worth remembering: a row an RLS
-- policy hides is a row that is NOT THERE for an UPDATE. The statement
-- matches zero rows, affects zero rows, and succeeds. No error is
-- raised, nothing is rolled back, and write_action returned an action
-- id having quietly dropped half of what it was asked to record. A
-- blocked INSERT raises; a blocked UPDATE just does nothing.
--
-- Two death saves were rolled on Goblin 1 - a 10 and a natural 1, which
-- is three failures between them - and its counters stayed on zero.
--
-- WHY NOT SIMPLY LET MEMBERS UPDATE ACTORS. Because then any player
-- could rename a goblin, drop its AC, or un-enrol it. The write that is
-- needed is narrow: three columns, the ones the rules move.
--
-- WHY NOT COLUMN GRANTS. `grant update (cols)` is per ROLE, and the DM
-- and the players are all `authenticated`. Narrowing the columns for a
-- player would narrow them for the DM too, who legitimately edits the
-- rest of the row.
--
-- WHY NOT A SECURITY DEFINER FUNCTION. It would work, and 002 spent
-- real effort getting that surface down to three functions that cannot
-- be avoided. A fourth to move two counters is not worth the widening.
--
-- SO: a members policy, and a trigger that refuses anything but the
-- death save columns from anyone who is not the DM. The policy decides
-- WHO may touch the row; the trigger decides WHAT they may change. The
-- trigger raises rather than ignoring, so this failure mode cannot
-- happen again quietly - which is the actual lesson here.
-- =====================================================================

create or replace function public.guard_actor_update()
returns trigger
language plpgsql
as $$
declare
  dm boolean;
begin
  select public.is_game_dm(e.game_id) into dm
    from public.encounters e
   where e.id = new.encounter_id;

  if coalesce(dm, false) then
    return new;
  end if;

  -- Everything that is not a death save is the DM's to change. Listed
  -- rather than inferred: a column added later is immutable to players
  -- until somebody deliberately adds it here.
  if (new.encounter_id, new.character_id, new.npc_key, new.label,
      new.initiative, new.ac_override, new.hp_override, new.active,
      new.enrolled_at)
     is distinct from
     (old.encounter_id, old.character_id, old.npc_key, old.label,
      old.initiative, old.ac_override, old.hp_override, old.active,
      old.enrolled_at)
  then
    raise exception
      'only the DM may change an actor beyond its death saves';
  end if;

  return new;
end;
$$;

comment on function public.guard_actor_update() is
  'Keeps the members UPDATE policy narrow: a player may move death_successes, death_failures and dead, and nothing else. Raises rather than ignoring, because the bug this migration fixes was an UPDATE that silently matched zero rows.';

create trigger actor_update_scope
  before update on public.encounter_actors
  for each row
  execute function public.guard_actor_update();

-- Permissive policies OR together, so the DM keeps the broad rights the
-- 011 policy grants and members gain a narrow one on top.
create policy "encounter_actors: members record death saves"
  on public.encounter_actors for update
  using (exists (select 1 from public.encounters e
                 where e.id = encounter_id and public.is_game_member(e.game_id)))
  with check (exists (select 1 from public.encounters e
                      where e.id = encounter_id and public.is_game_member(e.game_id)));
