-- =====================================================================
-- 049_this_one_is_different.sql
-- odyssey1e — everything about one object, editable on that object
-- =====================================================================
--
-- The object viewer shows nine facts and the editor reaches three. You
-- can rename a greatsword, change how many there are and say it is
-- unusually large, and that is the whole list - while the panel beside
-- it reports weight, damage, properties and kind that nothing can
-- touch.
--
-- ---------------------------------------------------------------------
-- WHY NOT A CATALOGUE ROW PER EDITED OBJECT
--
-- The obvious move is to write a campaign-scoped `items` row and point
-- the object at it. 008 did exactly that and said so without flinching:
-- "a +1 sword is not a sword", meaning a second catalogue entry.
--
-- 026 called that the wrong shape and it still is. It makes the
-- rulebook grow every time one weapon in one campaign gets enchanted,
-- and the catalogue stops being a list of what EXISTS and becomes a
-- list of what has ever happened. An edited object is a unique OBJECT,
-- which is what 026 built identity for.
--
-- ---------------------------------------------------------------------
-- WHY COLUMNS AND NOT ONE JSONB BAG
--
-- A bag would take any fact with no migration, and it was the first
-- instinct. Two things argued it down.
--
-- It is UNTYPED. `damage_denomination` is checked >= 2 on `items` and a
-- bag would carry `"denomination": 0` happily until something divided
-- by it. Every constraint the catalogue already has would need writing
-- again in Rust, and a rule written twice is 028.
--
-- And 036 ALREADY CHOSE, with `size_override` and `holds_size_override`.
-- Those are these columns; there are simply two of them so far. A bag
-- beside them is two mechanisms for one idea, which is the shape this
-- repo keeps having to undo - and folding them INTO a bag means
-- rewriting twenty-two call sites across six files that were written
-- yesterday.
--
-- SEVEN IS NOT FIFTEEN. The list below is what genuinely varies between
-- two objects of one kind, and it stops there on purpose. `kind` is not
-- here because a weapon that becomes armour is a different thing, not a
-- modified one, and `key` is not here because that IS the type. If this
-- ever passes ten, the bag wins and this comment is the argument for
-- switching.
--
-- NULL MEANS "AS THE CATALOGUE SAYS", every one of them. That is the
-- same tri-state `proficient_override` has used since 008: absent is
-- not the same as equal-to-the-default, because the default can change
-- underneath and the override should not.
-- =====================================================================

alter table public.objects add column weight_override              numeric
  check (weight_override is null or weight_override >= 0);
alter table public.objects add column price_override               integer
  check (price_override is null or price_override >= 0);
alter table public.objects add column damage_number_override       integer
  check (damage_number_override is null or damage_number_override >= 1);
alter table public.objects add column damage_denomination_override integer
  check (damage_denomination_override is null or damage_denomination_override >= 2);
alter table public.objects add column damage_types_override        text[];
alter table public.objects add column properties_override          text[];
alter table public.objects add column base_ac_override             integer
  check (base_ac_override is null or base_ac_override >= 0);

comment on column public.objects.weight_override is
  'What THIS one weighs, when it is not what the catalogue says. NULL means as the catalogue says - not "the same as the default", because the default can change underneath and an override should not follow it.';
comment on column public.objects.price_override is
  'What THIS one is worth, in its type''s own denomination. A shop prices from it, so a masterwork blade sells for more without a second catalogue row.';
comment on column public.objects.damage_number_override is
  'Dice COUNT for this one - 2 of a d6 where the type says 1. Checked >= 1 here rather than in Rust, because items.damage_number is checked here and a rule written twice is 028.';
comment on column public.objects.damage_denomination_override is
  'Die SIZE for this one. Checked >= 2 for the same reason the catalogue is: there is no d1, and a zero would be divided by.';
comment on column public.objects.damage_types_override is
  'What this one deals. A flaming sword is array[''slashing'',''fire''] and is still a longsword.';
comment on column public.objects.properties_override is
  'REPLACES the type''s properties rather than adding to them, which is the only reading that can REMOVE one - a balanced greatsword that lost `hvy` cannot be expressed by a list that only adds. So an override carries the full set.';
comment on column public.objects.base_ac_override is
  'Armour only. What this suit gives before Dexterity, when it is not what the type gives.';

-- The manager reads whole objects and the sheet reads a loadout; both
-- want every override in one go rather than a column at a time.
create index objects_overridden_idx on public.objects (id)
  where weight_override is not null
     or price_override is not null
     or damage_number_override is not null
     or damage_denomination_override is not null
     or damage_types_override is not null
     or properties_override is not null
     or base_ac_override is not null
     or size_override is not null
     or holds_size_override is not null;
