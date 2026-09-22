-- =====================================================================
-- 036_things_have_a_size.sql
-- odyssey1e — a greatsword does not go in a coin purse
-- =====================================================================
--
-- containers.rs has carried this in its header since 032, under "NOT
-- CARRIED OVER YET": `ItemSize` and a size limit, so a greatsword fails
-- to fit in a purse for a SECOND AND BETTER REASON than its tags. This
-- is that reason.
--
-- WHAT ALREADY EXISTS, because the answer here is a third axis and not
-- a replacement for either of the two 032 built:
--
--   accepts / content_tags   WHAT a container will take. A coin purse
--                            is {coin}. Empty means anything.
--   slots / capacity_slots   HOW MUCH room. A purse is 5 slots and a
--                            coin is 0.2, which is 25 coins.
--   weight                   Already on every one of the 68 catalogue
--                            rows, in pounds, and read by nothing.
--
-- Tags say a greatsword is not a coin. Slots say it is bulky. NEITHER
-- SAYS IT IS TOO BIG, and those are different objections: a marble is
-- untagged and tiny, and an unrestricted thimble should still refuse
-- it. Size is the one a person reaches for first, which is why it is
-- checked first.
--
-- ONE VOCABULARY, NOT A SECOND ONE THAT LOOKS LIKE IT. tiny sm med lg
-- huge grg, exactly as 010 spells it for creatures and 029 reads it for
-- hit dice. That is the lesson 033 took from a place `kind` that
-- drifted into holding a faction type, and the lesson 008 took from two
-- spellings of armour. A creature and a crate are measured on one
-- ladder, so "a Huge backpack holds Huge things" needs no translation.
--
-- WHY holds_size IS PERMISSIVE WHEN NULL, AND capacity_slots IS NOT.
-- They look alike and mean opposite things, which is worth saying out
-- loud rather than leaving to be discovered:
--
--   capacity_slots is a QUANTITY. Absent means nobody wrote it down,
--   which is an unfinished catalogue row, and 032 decided that reads as
--   a fault rather than as "bottomless".
--
--   holds_size is a RESTRICTION, like accepts. Absent means there is
--   not one - a sack cares how much you put in it and not how long any
--   one thing is. Empty accepts is already permissive for exactly this
--   reason, and a restriction that defaulted to refusing everything
--   would make every container a DM writes useless until they noticed.
--
-- THE OVERRIDES EARN THEIR PLACE, and they do it because there is no
-- screen for authoring a catalogue row. A giant's backpack that holds
-- Huge things and a rodent's that holds only Tiny ones are the same
-- catalogue key; without an override on the INSTANCE the only way to
-- have both is a migration. Same shape as ac_override, prof_bonus and
-- proficient_override: NULL means take it from the type.
--
-- WHAT IS DELIBERATELY NOT HERE: a weight capacity on containers.
-- slots is already the "how much fits" measure and is already
-- fractional for exactly that purpose. A second number meaning almost
-- the same thing is two numbers to keep in agreement, which is the debt
-- STATUS.md lists twice. Weight's own job is encumbrance - what a
-- CHARACTER can carry - and that is a rule about a person, not a sack.
-- =====================================================================

-- ---------------------------------------------------------------------
-- How big a thing is.
-- ---------------------------------------------------------------------

alter table public.items add column size text not null default 'med'
  check (size in ('tiny','sm','med','lg','huge','grg'));

comment on column public.items.size is
  'How big one of these is, on the SAME ladder 010 gives creatures: tiny sm med lg huge grg. Not a second vocabulary that looks like the first - a Huge backpack holding Huge things needs no translation. Defaults to med because most equipment is, and because a default of NULL would make every existing row unmeasurable rather than ordinary.';

-- ---------------------------------------------------------------------
-- The biggest thing a container will take.
-- ---------------------------------------------------------------------

alter table public.items add column holds_size text
  check (holds_size is null or holds_size in ('tiny','sm','med','lg','huge','grg'));

comment on column public.items.holds_size is
  'For kind=container: the largest size it admits. NULL MEANS NO SIZE LIMIT, the same way an empty accepts means any tag - it is a restriction, and an absent restriction is not one. That is the opposite of capacity_slots, where absent means the row is unfinished, because that one is a quantity. A sack cares how much goes in and not how long any one thing is.';

-- ---------------------------------------------------------------------
-- And the same two on the instance.
--
-- Because there is no screen for writing a catalogue row. A giant's
-- backpack and a rodent's are one key, and without these the only way
-- to have both is another migration.
-- ---------------------------------------------------------------------

alter table public.objects add column size_override text
  check (size_override is null or size_override in ('tiny','sm','med','lg','huge','grg'));
alter table public.objects add column holds_size_override text
  check (holds_size_override is null or holds_size_override in ('tiny','sm','med','lg','huge','grg'));

comment on column public.objects.size_override is
  'NULL means take it from the catalogue, which is what almost every object does. Here for the particular one that is not ordinary - a giant''s dagger is a shortsword to anybody else. Same shape as ac_override and prof_bonus.';
comment on column public.objects.holds_size_override is
  'NULL means take it from the catalogue. This is the giant''s backpack: one catalogue key, and the instance saying it swallows Huge things. NULL here does NOT mean unrestricted - it means ask the type, and the type''s own NULL is what means unrestricted.';

-- ---------------------------------------------------------------------
-- The catalogue, measured.
--
-- 5e HAS NO ITEM SIZE TABLE. The weights are the book's and were seeded
-- by 027 and 032; these sizes are a judgement, made from what a thing
-- IS rather than typed row by row, so the reasoning stays visible and a
-- DM who disagrees edits one row instead of guessing at a pattern.
-- ---------------------------------------------------------------------

-- Things you hold several of in one hand.
update public.items set size = 'tiny'
 where key in ('coin_cp','coin_sp','coin_gp','arrow','dart','sling',
               'dagger','tinderbox','holy_water','the_ember','coin_purse');

-- One-handed gear and most of the kit. Armour goes here because a suit
-- folds to roughly the bulk of the person who wears it.
update public.items set size = 'sm'
 where key in ('handaxe','light_hammer','sickle','club','mace','rations',
               'lamp','blanket','robe','clothes_fine','quiver','spell_book',
               'shortsword','scimitar','rapier','war_pick','morningstar',
               'crossbow_hand','shortbow','whip','net','trident','javelin');

-- The two-handed and the long: reach weapons, bows, greatweapons, and
-- the heaviest armour, which does not fold.
update public.items set size = 'lg'
 where key in ('greatsword','greataxe','maul','glaive','halberd','pike',
               'lance','longbow','heavy_crossbow','greatclub','quarterstaff',
               'spear','plate','splint','chain_mail');

-- A chest is furniture, not luggage.
update public.items set size = 'lg' where key = 'chest';

-- Everything untouched stays med, which is the honest answer for a
-- longsword, a battleaxe, a flail and a backpack.

-- ---------------------------------------------------------------------
-- What each container swallows.
--
-- A purse takes coins and nothing longer than one. A quiver takes
-- arrows. A backpack is the general case and stops at Medium, so a
-- greatsword does not go in one - which is the objection containers.rs
-- asked for, and the reason a sword is carried rather than packed.
-- ---------------------------------------------------------------------

update public.items set holds_size = 'tiny' where key in ('coin_purse','quiver');
update public.items set holds_size = 'sm'   where key in ('spell_book','priests_pack');
update public.items set holds_size = 'med'  where key = 'backpack';
update public.items set holds_size = 'lg'   where key = 'chest';
