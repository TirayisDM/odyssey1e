-- =====================================================================
-- 052_a_challenge_can_be_about_a_thing.sql
-- odyssey1e — the door in the room is the door you are forcing
-- =====================================================================
--
-- An encounter can be targeted at two things: a creature, which answers
-- with an AC, and a challenge, which answers with a DC. 011 built both
-- and `resolution.rs` makes exactly one distinction between them -
-- whether a natural decides it outright.
--
-- What is missing is the OBJECT. An encounter happens in a location
-- (034) and that location has things lying in it (033), and none of
-- them can be aimed at. The iron door is a challenge somebody typed the
-- name of, and the actual door - a row in `objects`, with a size and a
-- weight and a place - is invisible to the fight it is standing in.
--
-- ---------------------------------------------------------------------
-- A THIRD TARGET KIND WAS THE WRONG ANSWER
-- ---------------------------------------------------------------------
--
-- The obvious move is `objects` growing an AC and hit points so a thing
-- can be attacked. 5e has tables for exactly that - object AC by
-- material, hit points by size - and it is a real subsystem: hardness,
-- immunity to poison and psychic, thresholds. It is also not what makes
-- a door interesting at a table. Most of the time the question is "can
-- I get it open", which is a DC.
--
-- And a DC already exists. So an object becomes targetable by having a
-- CHALLENGE ATTACHED TO IT rather than by becoming a new kind of thing:
-- one mechanism, one resolver path, and `resolution.rs` unchanged.
--
-- The label still carries the act. "Force the door" is not the door; it
-- is a thing you do to the door, and two challenges can point at one
-- object - force it, pick it, listen at it - with different DCs and
-- different skills. That is why this is a column on the CHALLENGE and
-- not a flag on the object.
--
-- ATTACKING AN OBJECT IS DELIBERATELY STILL NOT POSSIBLE, and is worth
-- saying out loud so it is not mistaken for an oversight. It needs an
-- AC and hit points on objects and a rule about what breaking one does
-- to what is inside it. The blowgun and the net are the precedent: the
-- catalogue does not pretend to a rule the engine cannot roll.
--
-- ---------------------------------------------------------------------
-- SET NULL, NOT CASCADE
-- ---------------------------------------------------------------------
--
-- Destroying the chest must not delete the record that somebody tried
-- to pick it. The challenge survives with its label - "pick the iron
-- chest" still reads - and simply stops pointing anywhere, which is the
-- same decision 034 made about an encounter whose location is deleted.
-- =====================================================================

alter table public.encounter_challenges
  add column object_id uuid references public.objects(id) on delete set null;

comment on column public.encounter_challenges.object_id is
  'The thing this challenge is ABOUT, when it is about something real. NULL is the ordinary case and always will be - "notice the tripwire" is a challenge with no object behind it. Set, and the target list can show the door''s own name and a DM can see which of the things in the room has a difficulty on it. A column on the CHALLENGE rather than a flag on the object, because two challenges can point at one door - force it, pick it, listen at it - with different DCs.';

create index encounter_challenges_object_idx
  on public.encounter_challenges(object_id) where object_id is not null;

-- ---------------------------------------------------------------------
-- The thing has to be in the same game as the fight.
--
-- The fourth of these now - location_parent_same_game,
-- character_location_same_game, technique_object_same_game and this.
-- Same reason every time: a foreign key points at an id and cannot
-- check the pair, and a DM running two campaigns can see both sides.
--
-- NOT "in the same ROOM", deliberately. An encounter can have no
-- location at all (034 allows it), and a DM setting up a fight before
-- placing the furniture is ordinary. The game is the boundary that can
-- always be checked; the room is a judgement the DM is making.
-- ---------------------------------------------------------------------

create or replace function public.challenge_object_same_game()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
declare
  thing_game uuid;
  fight_game uuid;
begin
  if new.object_id is null then
    return new;
  end if;
  select game_id into thing_game from public.objects where id = new.object_id;
  select game_id into fight_game from public.encounters where id = new.encounter_id;
  if thing_game is distinct from fight_game then
    raise exception 'that thing is not in this campaign';
  end if;
  return new;
end;
$$;

comment on function public.challenge_object_same_game() is
  'Refuses a challenge pointed at an object in another campaign. The fourth trigger of this shape; a foreign key points at an id and cannot check the pair.';

create trigger challenges_are_about_our_own_things
  before insert or update of object_id on public.encounter_challenges
  for each row execute function public.challenge_object_same_game();
