-- =====================================================================
-- 098_a_people_can_be_hard_to_enchant.sql
-- odyssey1e — the Ny'ook's +2, decided and therefore built
-- =====================================================================
--
-- 097 seeded Magic and Toxin Resistance unapplied, because the source
-- said "+2 on saving throws" twice and "advantage" once and picking
-- one would have been inventing a rule. Dave picked: it is the +2.
--
-- A DECIDED RULE BELONGS IN THE ENGINE. Every other conditional trait
-- in this schema stays a DM call because the condition is something
-- nothing can detect - Stonecunning's doubling applies to SOME
-- stonework, Climber's Grace to SOME Athletics checks, and no request
-- says which. "Against a spell" is different in exactly one way that
-- matters: the player knows at the moment they roll, and this app has
-- always taken what they know as the request. `wis save` is already a
-- thing somebody types; `wis save vs spell` is the same sentence with
-- the circumstance in it.
--
-- A COLUMN, NOT A HARD-CODED TWO. The Ny'ook are the first people with
-- this and will not be the last, and a number on the row is what lets
-- the second one need no Rust at all - the same reasoning 076 used for
-- the audience ladder and 056 for ability bonuses.
--
-- NULL MEANS NO SUCH TRAIT, which is every other people. Not zero: a
-- species that genuinely grants +0 against spells is a thing somebody
-- could write down, and it should not read as having nothing.
--
-- WHAT THIS DOES NOT COVER. The trait says spells AND poisons. A
-- poison save is a CON save against something the engine also cannot
-- see, and there is no poison in the catalogue to save against - so
-- the column is named for what it does rather than for half of what
-- the trait says, and the poison half stays written down. When poison
-- arrives it gets its own column or this one grows a sibling; it does
-- not get folded in silently.
-- =====================================================================

alter table public.species
  add column if not exists spell_save_bonus integer;

comment on column public.species.spell_save_bonus is
  'A flat bonus this people adds to a saving throw AGAINST A SPELL, applied by resolve_request when the request names the circumstance - `wis save vs spell`. NULL means the people has no such trait, which is almost all of them; zero would mean one that grants nothing, which is a different claim. Poison saves are deliberately not covered: nothing in the catalogue is poisonous yet.';

update public.species
   set spell_save_bonus = 2
 where key = 'nyook' and game_id is null;

-- The trait stops hedging. 097 carried both readings because the
-- source did; this records the decision and marks it applied, because
-- from here the engine is the thing adding it.
update public.species
   set traits = jsonb_set(
         traits,
         array[(
           select (ordinality - 1)::int::text
             from jsonb_array_elements(traits) with ordinality t(v, ordinality)
            where v->>'name' = 'Magic and Toxin Resistance'
         )],
         '{"name": "Magic and Toxin Resistance",
           "text": "+2 on saving throws against spells, added by the engine when a save says what it is against - roll `wis save vs spell` rather than `wis save`. The same resistance covers poisons at the table, and that half stays a DM call: nothing in the catalogue is poisonous for the engine to recognise.",
           "applied": true}'::jsonb)
 where key = 'nyook' and game_id is null;
