-- =====================================================================
-- 050_this_sword_has_its_own_moves.sql
-- odyssey1e — a technique can belong to one object
-- =====================================================================
--
-- 049 made an object able to disagree with its type about nine facts.
-- This is the tenth thing, and it is not a column: what the object can
-- DO. A named sword should be able to carry a move nothing else has,
-- and editing one greatsword's Zwerchhau should not reach every
-- greatsword in the campaign.
--
-- ---------------------------------------------------------------------
-- A THIRD SCOPE ON THE PATTERN THE TABLE ALREADY USES
-- ---------------------------------------------------------------------
--
-- `techniques` is 001's nullable-tenancy shape: a surrogate id, a
-- nullable `game_id`, and two partial unique indexes on `key` - one for
-- the global rows and one per game. A game-scoped row shadows the
-- global row sharing its key. That is the same precedence `items`,
-- `npcs` and `narrative_lines` use.
--
-- So this adds a third level rather than a second mechanism:
--
--   global    game_id null, object_id null      the rulebook
--   game      game_id set,  object_id null      this campaign's version
--   object    game_id set,  object_id set       this one thing
--
-- AN OBJECT ROW CARRIES ITS GAME TOO, and that is not redundant. Every
-- write policy on this table reads `game_id is not null and
-- is_game_dm(game_id)`, so a row without one is writable by nobody. The
-- CHECK below makes that structural rather than a thing to remember.
--
-- AND THE GAME INDEX HAD TO NARROW. `techniques_game_key_idx` is unique
-- on (key, game_id) where game_id is not null - which an object row
-- also satisfies, so two swords in one campaign overriding the same key
-- would collide. It is recreated excluding object rows, which get their
-- own index on (key, object_id). Dropping and recreating an index in a
-- later migration is the 003/004 pattern: a fix, not an edit.
--
-- ---------------------------------------------------------------------
-- REMOVAL NEEDS A TOMBSTONE, AND HERE IS WHY IT CANNOT BE A DELETE
-- ---------------------------------------------------------------------
--
-- Deleting an object's row means "stop overriding", which restores the
-- inherited technique. There is no row to delete to express "this sword
-- does NOT have the move its type has" - the row belongs to the type
-- and deleting THAT would take it from every other greatsword.
--
-- So `removed` is a flag on an object-scoped row: the row exists, it
-- shadows the inherited one, and it says the answer is nothing. Same
-- reasoning 049 gave for lists replacing rather than merging - the only
-- reading that can REMOVE something - and the same reasoning 015 gave
-- for the dying condition being derived from events rather than
-- stored: the absence has to be representable.
--
-- It is refused on the other two scopes. A global tombstone would mean
-- the rulebook deleting its own row, and a campaign one is a real idea
-- - "no weapon in this game has Zwerchhau" - that nobody has asked for
-- and that would need its own thought about what a player sees.
--
-- A TOMBSTONE FILLS THE NOT NULL COLUMNS by copying the parent, which
-- looks wasteful and is deliberate: un-removing is then one flag rather
-- than a reconstruction, and a tombstone that lost the original numbers
-- could not be undone.
--
-- ---------------------------------------------------------------------
-- WHAT IS DELIBERATELY NOT HERE: a per-object technique cannot be given
-- to a different item_key than the object's own. Nothing enforces that
-- yet because nothing can express it - the editor writes the object's
-- key - and a constraint reaching from techniques into objects to check
-- it is a join a CHECK cannot do. If it ever matters it is a trigger,
-- the same shape as location_parent_same_game.
-- =====================================================================

alter table public.techniques
  add column object_id uuid references public.objects(id) on delete cascade;

alter table public.techniques
  add column removed boolean not null default false;

comment on column public.techniques.object_id is
  'Whose move this is, when it belongs to ONE object rather than to a type. NULL is the ordinary case: the row applies to every object of its item_key. Set, and it shadows the game and global rows sharing its key for that object alone - 050 added the third scope onto 001''s nullable-tenancy pattern rather than inventing a second mechanism.';
comment on column public.techniques.removed is
  'A TOMBSTONE, and only ever on an object-scoped row. Deleting an object''s row means "stop overriding" and restores the inherited technique; there is no row to delete to say "this sword does not have the move its type has", because that row belongs to the type. So the absence gets a representation. The NOT NULL columns are copied from the parent so that un-removing is one flag rather than a reconstruction.';

-- An object row must carry its game, or every write policy on this
-- table refuses it - they all read game_id.
alter table public.techniques
  add constraint techniques_object_needs_a_game
  check (object_id is null or game_id is not null);

-- A tombstone means nothing at the other two scopes. See the header.
alter table public.techniques
  add constraint techniques_only_objects_are_removed
  check (removed = false or object_id is not null);

-- ---------------------------------------------------------------------
-- The game index excludes object rows, which get their own.
-- ---------------------------------------------------------------------

drop index if exists public.techniques_game_key_idx;
create unique index techniques_game_key_idx
  on public.techniques(key, game_id)
  where game_id is not null and object_id is null;

create unique index techniques_object_key_idx
  on public.techniques(key, object_id)
  where object_id is not null;

create index techniques_object_idx
  on public.techniques(object_id) where object_id is not null;

-- ---------------------------------------------------------------------
-- An object's technique belongs to the object's own game.
--
-- The mirror of location_parent_same_game and
-- character_location_same_game, and it exists for the same reason: a
-- foreign key points at an id and cannot check the pair. A DM running
-- two campaigns can see both, and is exactly the person who would cross
-- them by accident.
-- ---------------------------------------------------------------------

create or replace function public.technique_object_same_game()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
declare
  owner_game uuid;
begin
  if new.object_id is null then
    return new;
  end if;
  select game_id into owner_game from public.objects where id = new.object_id;
  if owner_game is distinct from new.game_id then
    raise exception 'a technique and the object it belongs to must be in the same game';
  end if;
  return new;
end;
$$;

comment on function public.technique_object_same_game() is
  'Refuses an object-scoped technique whose game does not match the object''s. Mirrors location_parent_same_game, and exists for the same reason: a foreign key points at an id and cannot check the pair.';

create trigger techniques_belong_to_their_objects_game
  before insert or update of object_id, game_id on public.techniques
  for each row execute function public.technique_object_same_game();
