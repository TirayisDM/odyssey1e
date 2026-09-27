-- =====================================================================
-- 058_what_a_character_looks_like.sql
-- odyssey1e — the body, the tongues, and the things a people costs you
-- =====================================================================
--
-- 056 and 057 gave a SPECIES a height band, a set of traits and a list
-- of languages. None of that describes a PERSON. The Unt'garoth run 7
-- to 10 feet and weigh 400 to 900 pounds; Garn is one specific height
-- and one specific weight, and until now there was nowhere to say so.
--
-- THE SPECIES STATES A RANGE, THE CHARACTER STATES A VALUE. That is the
-- whole shape of this migration. `species.height_min_ft` is what the
-- people are; `characters.height_ft` is what this one is, and a screen
-- can show the second against the first and say whether it is unusual.
-- Same pattern 049 used for object overrides and 056 for ability
-- bonuses: the general fact and the particular one stay apart so both
-- can be true.
--
-- ---------------------------------------------------------------------
-- SPOKEN AND WRITTEN ARE DIFFERENT FACTS
-- ---------------------------------------------------------------------
--
-- `languages text[]` could only say a name. 5e writes "speak, read and
-- write X" as one phrase and most species do have all three, but the
-- interesting cases are the ones that do not: a tongue with no script,
-- a scholar who reads a dead language they cannot pronounce, an
-- illiterate character who speaks four.
--
-- So a language is now an object - name, spoken, written - and the two
-- booleans are separate because they are separately true. A text[] of
-- names could never have carried that, and bolting a second array
-- beside it would have been two lists that can disagree about which
-- entry is which.
--
-- A CHARACTER HAS THEIR OWN, on top of their people's. Their species
-- supplies a baseline and a life supplies the rest; the panel shows the
-- union and says which came from where.
--
-- ---------------------------------------------------------------------
-- A TRAIT CAN COST YOU SOMETHING
-- ---------------------------------------------------------------------
--
-- 056's traits were all upside, because the two species seeded first
-- mostly are. They are not all going to be. An Unt'garoth CANNOT SWIM -
-- their own document says the density does not permit it - and showing
-- that in the same list, in the same colour, as "resistance to fire and
-- cold" is a screen lying by arrangement.
--
-- `kind` is 'feature' or 'drawback' and defaults to feature, so every
-- trait already written stays what it was. This is deliberately NOT a
-- mechanical distinction - nothing computes differently - it is a
-- presentational one, and that is the honest description of it.
--
-- Only ONE drawback is marked here, the Unt'garoth's Dense Mass,
-- because it is the only one either document actually states. The
-- Unt'gar have none written down. Inventing some to balance the two
-- would be designing Dave's game for him.
-- =====================================================================

-- ---------------------------------------------------------------------
-- THE BODY
-- ---------------------------------------------------------------------

alter table public.characters
  add column if not exists height_ft   numeric,
  add column if not exists weight_lb   numeric,
  add column if not exists hair        text,
  add column if not exists skin        text,
  add column if not exists eyes        text,
  add column if not exists description text;

comment on column public.characters.height_ft is
  'This character''s actual height in feet. The SPECIES carries a band (species.height_min_ft); this is where in it they fall, and a screen can say when that is unusual.';
comment on column public.characters.weight_lb is
  'Pounds. Not connected to carrying capacity - that is Strength and size. This is what the character weighs, which matters for being carried, falling, and thin ice.';
comment on column public.characters.hair is 'Free text. A species suggests; a person decides.';
comment on column public.characters.skin is 'Free text.';
comment on column public.characters.eyes is 'Free text.';
comment on column public.characters.description is
  'Anything else worth seeing at a glance - scars, bearing, the missing finger. Prose the engine will never read, on the same contract as 043''s special text and 053''s narrative.';

alter table public.characters
  drop constraint if exists characters_body_is_positive;
alter table public.characters
  add constraint characters_body_is_positive check (
    (height_ft is null or height_ft > 0)
    and (weight_lb is null or weight_lb > 0)
  );

-- ---------------------------------------------------------------------
-- THE TONGUES
-- ---------------------------------------------------------------------

alter table public.characters
  add column if not exists languages jsonb not null default '[]'::jsonb;

comment on column public.characters.languages is
  'Languages this character knows BEYOND their species'' own, as [{name, spoken, written}]. Spoken and written are separate because they are separately true - a dead language can be read and not pronounced.';

alter table public.characters
  drop constraint if exists characters_languages_is_a_list;
alter table public.characters
  add constraint characters_languages_is_a_list
    check (jsonb_typeof(languages) = 'array');

-- The species side, converted from a bare list of names.
alter table public.species
  add column if not exists tongues jsonb not null default '[]'::jsonb;

comment on column public.species.tongues is
  'What this people speaks and writes, as [{name, spoken, written}]. Replaces the `languages` text[], which could only carry a name and so could not say that a tongue has no script.';

alter table public.species
  drop constraint if exists species_tongues_is_a_list;
alter table public.species
  add constraint species_tongues_is_a_list
    check (jsonb_typeof(tongues) = 'array');

-- Both seeded peoples speak, read and write everything on their list,
-- which is what their documents say. Converted from the old column
-- rather than retyped, so a name cannot drift in the copying.
update public.species s
   set tongues = (
     select coalesce(jsonb_agg(jsonb_build_object(
              'name', l, 'spoken', true, 'written', true)), '[]'::jsonb)
     from unnest(s.languages) as l
   )
 where jsonb_array_length(s.tongues) = 0;

-- ---------------------------------------------------------------------
-- WHAT A PEOPLE COSTS YOU
-- ---------------------------------------------------------------------
--
-- Every existing trait becomes a feature, which is what they all were.

update public.species
   set traits = (
     select jsonb_agg(
       case when t ? 'kind' then t
            else t || '{"kind":"feature"}'::jsonb end
       order by ord)
     from jsonb_array_elements(traits) with ordinality as e(t, ord)
   )
 where jsonb_array_length(traits) > 0;

-- The one either document actually states. An Unt'garoth cannot swim.
update public.species
   set traits = (
     select jsonb_agg(
       case when t->>'name' = 'Dense Mass'
            then t || '{"kind":"drawback"}'::jsonb
            else t end
       order by ord)
     from jsonb_array_elements(traits) with ordinality as e(t, ord)
   )
 where key = 'untgaroth' and game_id is null;
