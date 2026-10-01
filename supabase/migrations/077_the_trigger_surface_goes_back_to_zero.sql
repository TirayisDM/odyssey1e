-- =====================================================================
-- 077_the_trigger_surface_goes_back_to_zero.sql
-- odyssey1e — finishing what 002 and 017 started, fifteen times over
-- =====================================================================
--
-- 002 revoked the EXECUTE that Postgres grants to PUBLIC by default on
-- every function in this schema, and 017 did it again for the ones 009
-- through 016 had added since. Both migrations said the same thing: a
-- SECURITY DEFINER function runs as its owner, so the surface is every
-- one of them that anybody can call, and the surface should be small
-- enough to read.
--
-- It is not small any more. Twenty trigger functions exist; FIVE carry
-- the revoke and fifteen do not, because the pattern lived in two
-- migrations rather than in anything that could notice its absence.
-- Supabase's own advisor has been reporting them the whole time, and
-- STATUS.md has said "three SECURITY DEFINER advisor warnings are
-- expected" since the day there were three. An expectation that is
-- stale is worse than no expectation: fifteen warnings against a note
-- promising three teaches a reader to stop opening the list.
--
-- ---------------------------------------------------------------------
-- WHAT IS ACTUALLY AT RISK, STATED HONESTLY
-- ---------------------------------------------------------------------
--
-- Very little, and that is not a reason to leave it. Postgres refuses
-- to execute a trigger function called any other way - "trigger
-- functions can only be called as triggers" - so a signed-in player
-- hitting /rest/v1/rpc/sync_character_level gets an error rather than
-- a free level. The grant is reachable, not useful.
--
-- It is removed because the surface is supposed to be auditable. A list
-- of fifteen reachable functions that happen to be harmless is a list
-- nobody checks, and the sixteenth is the one that is not a trigger.
--
-- THE FIVE THAT STAY ARE THE DELIBERATE ONES, and they are not
-- triggers: is_game_member and is_game_dm are called BY the policies
-- and must be executable by `authenticated` or every policy in 001
-- fails closed; holder_character and holder_is_a_location are the same
-- story for the object policies; join_game is the one a player calls on
-- purpose. None is callable by `anon`. Those are the advisor warnings
-- that are genuinely expected, and STATUS now says five and names them.
--
-- ---------------------------------------------------------------------
-- AND THREE FUNCTIONS THAT PIN THE WRONG search_path
-- ---------------------------------------------------------------------
--
-- Twenty-two of the twenty-five pin `search_path = ''` and write every
-- table name out in full. Three - all of them recent - pin `public`:
--
--   061  stamp_action_cost
--   063  held_actor_same_encounter
--   073  sync_character_level
--
-- With an empty search_path a name that is not schema-qualified simply
-- fails to resolve, which turns a typo into an error at creation time.
-- With `public` it resolves - and the temporary schema is still
-- searched ahead of it for tables, so a session holding a temp table
-- named `characters` would have sync_character_level update that
-- instead. Writing to the wrong table and reporting success is this
-- codebase's named defect class, and it is worth closing even where
-- reaching it needs a direct database connection that no player has.
--
-- The first two already write `public.` in full and only need the
-- setting changed. The third does not, so it is replaced whole.
-- =====================================================================

-- ---------------------------------------------------------------------
-- The fifteen.
-- ---------------------------------------------------------------------

revoke all on function public.challenge_object_same_game() from public, anon, authenticated;
revoke all on function public.character_gets_an_entity() from public, anon, authenticated;
revoke all on function public.character_location_same_game() from public, anon, authenticated;
revoke all on function public.container_gets_an_entity() from public, anon, authenticated;
revoke all on function public.entity_goes_with_its_owner() from public, anon, authenticated;
revoke all on function public.held_actor_same_encounter() from public, anon, authenticated;
revoke all on function public.location_gets_an_entity() from public, anon, authenticated;
revoke all on function public.location_parent_same_game() from public, anon, authenticated;
revoke all on function public.no_container_cycles() from public, anon, authenticated;
revoke all on function public.no_location_cycles() from public, anon, authenticated;
revoke all on function public.stamp_action_cost() from public, anon, authenticated;
revoke all on function public.sync_character_level() from public, anon, authenticated;
revoke all on function public.technique_object_same_game() from public, anon, authenticated;
revoke all on function public.turn_actor_is_in_the_encounter() from public, anon, authenticated;
revoke all on function public.unheld_is_unequipped() from public, anon, authenticated;

-- ---------------------------------------------------------------------
-- The three search paths.
--
-- A trigger keeps firing across an ALTER - the trigger points at the
-- function, and this changes a setting on it rather than replacing it.
-- ---------------------------------------------------------------------

alter function public.stamp_action_cost() set search_path = '';
alter function public.held_actor_same_encounter() set search_path = '';

-- Replaced rather than altered: its body names `character_classes` and
-- `characters` bare, and an empty search_path would stop both
-- resolving. Same rule as 073 wrote it, every name now in full.
create or replace function public.sync_character_level()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
declare
  who   uuid;
  tally integer;
  lead_key text;
begin
  who := coalesce(new.character_id, old.character_id);

  select sum(level) into tally
    from public.character_classes where character_id = who;

  -- `class_key` LAST IN THE ORDER, so two classes added in the same
  -- transaction - which share a default now() - still resolve to one
  -- answer rather than whichever the planner happened to return.
  -- multiclass::primary breaks the same tie the same way.
  select class_key into lead_key
    from public.character_classes
   where character_id = who
   order by level desc, added_at asc, class_key asc
   limit 1;

  update public.characters c
     set level = coalesce(tally, c.level),
         class_key = lead_key
   where c.id = who;

  return null;
end
$$;

revoke all on function public.sync_character_level() from public, anon, authenticated;
