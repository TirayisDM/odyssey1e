-- =====================================================================
-- 035_characters_stand_somewhere.sql
-- odyssey1e — a person is in a room
-- =====================================================================
--
-- 033 gave the world places and 034 put encounters in them. Objects rest
-- in a location through holder_id, and a fight happens at one. The thing
-- still missing is the obvious one: WHERE SOMEBODY IS.
--
-- The scene view asks three questions of a place - who is here, what is
-- happening here, what is lying here - and two of them could already be
-- answered. This is the third.
--
-- WHY A COLUMN AND NOT A DERIVATION. Everything else added since 033 has
-- been worked out rather than stored: depth, path, AC, the dying
-- condition, a challenge's status. The rule there is that a number
-- implied by other facts must not be written down, because a written
-- copy is one edit away from lying.
--
-- Where a person is standing is not implied by anything. It is the
-- fact. The only near-miss is enrolment - a goblin in the Frostvalley
-- brawl is arguably in the Frostvalley Inn - and that derivation breaks
-- the moment it matters: a character in no encounter is nowhere, an
-- encounter with no location tells you nothing, and the goblin that
-- survives the fight teleports back to nowhere when the DM ends it.
-- Presence outlives the fight it was first noticed in.
--
-- SET NULL, NOT RESTRICT, and the opposite of what parent_id does.
-- Deleting a continent must not silently delete its settlements, so that
-- one refuses. Deleting a room must not refuse because someone is
-- standing in it - people are not contents. They end up nowhere, which
-- is where everybody already is today, and they can be put back.
--
-- NULL IS A REAL ANSWER and it is the default for every row that exists.
-- A character off-screen, between scenes, or simply never placed is
-- nowhere in particular. This is not a migration anyone has to complete.
--
-- NO NEW POLICY. 001's "characters: owner or dm updates" already says
-- who may move a character: the player who owns it, or the DM. That is
-- the right answer for walking into the next room, and it needed no
-- change to become it.
--
-- WHAT IS DELIBERATELY NOT HERE: travel time, distance, adjacency,
-- routes, line of sight, who can see whom. 033 refused coordinates for
-- the reason the original earned - position was defined up front and is
-- still null years later. This is presence, which is a single id, and
-- movement between places is a walk the DM narrates.
-- =====================================================================

alter table public.characters
  add column location_id uuid references public.locations(id) on delete set null;

comment on column public.characters.location_id is
  'Where this person is. NULL means nowhere in particular, which is a real answer and the default - a character between scenes is not misfiled. SET NULL on delete because people are not contents: removing a room should not refuse because someone is standing in it, unlike parent_id which restricts. Not derived from encounter enrolment, because presence outlives the fight it was first noticed in.';

create index characters_location_idx
  on public.characters(location_id) where location_id is not null;

-- ---------------------------------------------------------------------
-- A person cannot stand in another campaign's room.
--
-- The mirror of location_parent_same_game, and it exists for the same
-- reason: a foreign key points at an id, not at the pair, so nothing in
-- the FK can notice that the character and the place belong to
-- different games. RLS would usually stop it - you cannot see a room in
-- a game you are not in - but a DM who runs two campaigns can see both,
-- and that is precisely the person who would do this by accident.
-- ---------------------------------------------------------------------

create or replace function public.character_location_same_game()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
declare
  place_game uuid;
begin
  if new.location_id is null then
    return new;
  end if;
  select game_id into place_game from public.locations where id = new.location_id;
  if place_game is distinct from new.game_id then
    raise exception 'a character and the place they are standing in must be in the same game';
  end if;
  return new;
end;
$$;

comment on function public.character_location_same_game() is
  'Refuses to stand a character in a place belonging to a different game. Mirrors location_parent_same_game, and exists for the same reason: a foreign key points at an id and cannot check the pair.';

create trigger characters_stand_in_their_own_game
  before insert or update of location_id on public.characters
  for each row execute function public.character_location_same_game();
