-- =====================================================================
-- 007_technique_item_keys.sql
-- odyssey1e — techniques stop naming their weapon in prose
-- =====================================================================
--
-- THE DEFECT
--
-- 006 seeded techniques.weapon as free text holding a display name:
-- 'light hammer (thrown)'. That column's own comment admitted the shape
-- was provisional — "Matching against inventory is by name for now."
-- Two separate things are wrong with it.
--
-- A display name is not an identifier. Rename a weapon and every
-- technique pointing at it stops matching, silently, with no error and
-- no dangling reference to notice. The techniques simply stop existing
-- as far as the resolver is concerned.
--
-- And the string overloads two facts into one field: WHICH ITEM, and
-- WHICH ATTACK MODE. 'light hammer' and 'light hammer (thrown)' are one
-- weapon used two ways, and they carry different technique lists — 7
-- melee, 6 thrown. Parsing that distinction back out of a display name
-- means parsing parentheses, which is not a thing a schema should ask
-- anyone to do.
--
-- THE FIX
--
-- Two columns doing one job each. item_key names the item by value;
-- mode says how the item is being used.
--
-- WHY NOT A FOREIGN KEY
--
-- Same reason dice_faces.set_key is not one, and character_skills
-- .skill_key before it. Reference tables here carry a nullable game_id:
-- a global row and a campaign override share a key and are told apart
-- by game_id. So key is unique only WITHIN each partition, enforced by
-- two partial unique indexes — and PostgreSQL will not let a foreign
-- key reference a partial unique index. An FK on the surrogate id would
-- work mechanically but would defeat the override model: it would bind
-- each technique to either the global item or one campaign's version,
-- when the whole point is that the key resolves to whichever the
-- reader is entitled to see. By value is not a shortcut here; it is
-- the pattern this schema is built on.
--
-- WHY THIS LANDS BEFORE THE ITEMS TABLE
--
-- Precisely because there is no FK, there is no DDL dependency. The
-- repoint stands on its own and can be reviewed on its own. What it
-- does do is MINT the item key vocabulary, and the items table when it
-- arrives must use these keys exactly:
--
--   mace_of_the_deep_song
--   light_hammer
--   heavy_crossbow
--
-- Slugs are the item's display name lowercased with every run of
-- non-alphanumeric characters collapsed to one underscore. The mace
-- keeps its full name rather than its Foundry baseItem ('mace')
-- because it is a specific artifact, not a generic mace: Crystal
-- Resonance and Deepsong Echo belong to that weapon, not to the type.
--
-- Nothing in the codebase reads techniques yet, so no Rust or JS
-- changes ship with this.
-- =====================================================================

alter table public.techniques add column item_key text;
alter table public.techniques add column mode     text;

-- The whole mapping. Four source values; 'light hammer' splits into two
-- modes of one item, which is the overload this migration exists to undo.
update public.techniques set item_key = 'mace_of_the_deep_song', mode = 'melee'
  where weapon = 'mace of the deep song';
update public.techniques set item_key = 'heavy_crossbow',        mode = 'ranged'
  where weapon = 'heavy crossbow';
update public.techniques set item_key = 'light_hammer',          mode = 'melee'
  where weapon = 'light hammer';
update public.techniques set item_key = 'light_hammer',          mode = 'thrown'
  where weapon = 'light hammer (thrown)';

-- Any weapon string the mapping above did not anticipate leaves nulls
-- behind. Fail loudly here rather than dropping the column and losing
-- the evidence of what went unmapped.
do $$
declare
  unmapped integer;
  sample   text;
begin
  select count(*) into unmapped
    from public.techniques where item_key is null or mode is null;

  if unmapped > 0 then
    select string_agg(distinct coalesce(weapon, '<null>'), ', ')
      into sample
      from public.techniques where item_key is null or mode is null;
    raise exception
      '007: % technique row(s) had an unmapped weapon value: %', unmapped, sample;
  end if;
end $$;

alter table public.techniques alter column item_key set not null;
alter table public.techniques alter column mode     set not null;

alter table public.techniques
  add constraint techniques_mode_check
  check (mode in ('melee', 'thrown', 'ranged'));

alter table public.techniques drop column weapon;

-- Every lookup is "what can this weapon do in this mode", so index the
-- pair. Not unique: a weapon-mode has many techniques, which is the
-- entire point of the table.
create index techniques_item_mode_idx
  on public.techniques(item_key, mode);

comment on column public.techniques.item_key is
  'References items.key by value, not by FK — a global item and a game override share a key, so an FK could not point at one of them unambiguously. Same reasoning as dice_faces.set_key. Replaces the free-text weapon column from 006.';
comment on column public.techniques.mode is
  'How the weapon is being used: melee, thrown or ranged. A light hammer has separate technique lists for melee and thrown, so the mode is part of what a technique attaches to, not a display detail. Derived from weapon properties at resolve time (thr grants a thrown mode); stored here because a technique belongs to one mode.';
