-- =====================================================================
-- 027_the_catalogue.sql
-- odyssey1e — the rest of the weapons and armour
-- =====================================================================
--
-- `items` had fifteen rows, and every one of them was there because
-- something else needed it. Rodnar's kit put a mace and a crossbow in;
-- 019 and 020 added a handaxe and a scimitar because a goblin had to
-- swing something. That is how a catalogue gets holes: it grows to
-- satisfy whoever asked last, and nobody ever asks for a sickle.
--
-- Now that 026 gave objects an identity there is somewhere to put a
-- thing, so this fills in the type side: the SRD simple and martial
-- weapons, and the whole armour table. Global rows - game_id null - so
-- every campaign reads them, and a campaign that wants a different
-- longsword overrides the key rather than editing this.
--
-- ALREADY PRESENT AND NOT REPEATED: handaxe, light_hammer, scimitar,
-- heavy_crossbow, scale_mail, mace_of_the_deep_song. The first five are
-- ordinary catalogue rows that happened to arrive early. The sixth is
-- not a mace - it is Rodnar's, and the plain `mace` below is the type
-- it should have been made from.
--
-- ---------------------------------------------------------------------
-- VERSATILE NEEDED TWO COLUMNS, and this is the honest half of the fix.
--
-- A longsword is 1d8 in one hand and 1d10 in two. There was nowhere to
-- write the second number, so the choice was a wrong catalogue row or a
-- place to put the fact. This adds the place.
--
-- NOTHING READS THEM YET. equipment::modes() derives modes from
-- properties and knows melee, thrown and ranged, because that is what
-- `techniques_mode_check` allows; a Versatile mode means widening that
-- constraint and teaching the attack resolver which die to take. That
-- is a rule change and it belongs in its own migration with its own
-- tests. What this does is stop the catalogue from lying in the
-- meantime: the row says 1d8 with `ver` 1d10, and today the engine
-- offers the 1d8.
--
-- STRENGTH REQUIREMENTS ARE NOT HERE. Chain mail wants Str 13 and plate
-- wants 15, and there is no column for it. Same reasoning: a fact with
-- no rule reading it is a column waiting to go stale. It goes in when
-- the encumbrance rule does.
--
-- THE BLOWGUN IS MISSING, on purpose. It does a flat 1 piercing, and
-- damage_denomination is checked >= 2 - there is no such die. Writing
-- 1d2 would make it a different weapon, and relaxing the check to admit
-- 1d1 would put a die that cannot vary into a dice system. It waits for
-- a flat-damage column, which several other things will want too.
-- =====================================================================

alter table public.items add column versatile_number       integer
  check (versatile_number is null or versatile_number >= 1);
alter table public.items add column versatile_denomination integer
  check (versatile_denomination is null or versatile_denomination >= 2);

comment on column public.items.versatile_denomination is
  'The two-handed die of a versatile weapon - 10 on a longsword, whose damage_denomination is 8. Null on everything else. NOTHING READS THIS YET: there is no Versatile mode, because techniques_mode_check allows three and widening it is a rule change. See 027 for why the fact is stored anyway.';
comment on column public.items.versatile_number is
  'Dice count for the two-handed die. 1 on every SRD versatile weapon; the column exists so 2d6 is expressible rather than assumed away.';

-- ---------------------------------------------------------------------
-- Weapons.
--
-- Property codes are the same three-letter forms 008 chose and
-- equipment.rs reads: lgt light, hvy heavy, fin finesse, thr thrown,
-- two two-handed, amm ammunition, lod loading, ver versatile, rch
-- reach, spc special.
--
-- `rch` and `spc` are NEW as data and inert as rules. Reach is a grid
-- fact and there is no grid; special means "read the entry", and the
-- entry is prose the engine cannot act on. They are here so a glaive
-- can be told from a greatsword before either rule exists.
--
-- range_reach is in inches, following the mace: 8 is a five-foot reach,
-- so a reach weapon is 16. range_value and range_long are in feet,
-- following the crossbow. That inconsistency is 008's, and this is not
-- the migration that fixes it.
-- ---------------------------------------------------------------------

insert into public.items
  (key, game_id, name, kind, base_item,
   weapon_class, damage_number, damage_denomination, damage_types, properties,
   versatile_number, versatile_denomination,
   range_reach, range_value, range_long,
   rarity, price, denom, weight)
values
  -- simple melee
  ('club',         null, 'Club',         'weapon', 'club',         'simpleM', 1,  4, array['bludgeoning']::text[], array['lgt']::text[],             null, null, null, null, null, null,  1, 'sp',  2),
  ('dagger',       null, 'Dagger',       'weapon', 'dagger',       'simpleM', 1,  4, array['piercing']::text[],    array['fin','lgt','thr']::text[], null, null, null,   20,   60, null,  2, 'gp',  1),
  ('greatclub',    null, 'Greatclub',    'weapon', 'greatclub',    'simpleM', 1,  8, array['bludgeoning']::text[], array['two']::text[],             null, null, null, null, null, null,  2, 'sp', 10),
  ('javelin',      null, 'Javelin',      'weapon', 'javelin',      'simpleM', 1,  6, array['piercing']::text[],    array['thr']::text[],             null, null, null,   30,  120, null,  5, 'sp',  2),
  ('mace',         null, 'Mace',         'weapon', 'mace',         'simpleM', 1,  6, array['bludgeoning']::text[], '{}',                             null, null,    8, null, null, null,  5, 'gp',  4),
  ('quarterstaff', null, 'Quarterstaff', 'weapon', 'quarterstaff', 'simpleM', 1,  6, array['bludgeoning']::text[], array['ver']::text[],                1,    8, null, null, null, null,  2, 'sp',  4),
  ('sickle',       null, 'Sickle',       'weapon', 'sickle',       'simpleM', 1,  4, array['slashing']::text[],    array['lgt']::text[],             null, null, null, null, null, null,  1, 'gp',  2),
  ('spear',        null, 'Spear',        'weapon', 'spear',        'simpleM', 1,  6, array['piercing']::text[],    array['thr','ver']::text[],          1,    8, null,   20,   60, null,  1, 'gp',  3),

  -- simple ranged
  ('crossbow_light', null, 'Crossbow, Light', 'weapon', 'lightcrossbow', 'simpleR', 1, 8, array['piercing']::text[],    array['amm','lod','two']::text[], null, null, null,  80, 320, null, 25, 'gp', 5),
  ('dart',           null, 'Dart',            'weapon', 'dart',          'simpleR', 1, 4, array['piercing']::text[],    array['fin','thr']::text[],       null, null, null,  20,  60, null,  5, 'cp', 0.25),
  ('shortbow',       null, 'Shortbow',        'weapon', 'shortbow',      'simpleR', 1, 6, array['piercing']::text[],    array['amm','two']::text[],       null, null, null,  80, 320, null, 25, 'gp', 2),
  ('sling',          null, 'Sling',           'weapon', 'sling',         'simpleR', 1, 4, array['bludgeoning']::text[], array['amm']::text[],             null, null, null,  30, 120, null,  1, 'sp', 0),

  -- martial melee
  ('battleaxe',   null, 'Battleaxe',   'weapon', 'battleaxe',   'martialM', 1,  8, array['slashing']::text[],    array['ver']::text[],                     1,   10, null, null, null, null,  10, 'gp',  4),
  ('flail',       null, 'Flail',       'weapon', 'flail',       'martialM', 1,  8, array['bludgeoning']::text[], '{}',                                  null, null, null, null, null, null,  10, 'gp',  5),
  ('glaive',      null, 'Glaive',      'weapon', 'glaive',      'martialM', 1, 10, array['slashing']::text[],    array['hvy','rch','two']::text[],      null, null,   16, null, null, null,  20, 'gp',  6),
  ('greataxe',    null, 'Greataxe',    'weapon', 'greataxe',    'martialM', 1, 12, array['slashing']::text[],    array['hvy','two']::text[],            null, null, null, null, null, null,  30, 'gp',  7),
  ('greatsword',  null, 'Greatsword',  'weapon', 'greatsword',  'martialM', 2,  6, array['slashing']::text[],    array['hvy','two']::text[],            null, null, null, null, null, null,  50, 'gp',  6),
  ('halberd',     null, 'Halberd',     'weapon', 'halberd',     'martialM', 1, 10, array['slashing']::text[],    array['hvy','rch','two']::text[],      null, null,   16, null, null, null,  20, 'gp',  6),
  ('lance',       null, 'Lance',       'weapon', 'lance',       'martialM', 1, 12, array['piercing']::text[],    array['rch','spc']::text[],            null, null,   16, null, null, null,  10, 'gp',  6),
  ('longsword',   null, 'Longsword',   'weapon', 'longsword',   'martialM', 1,  8, array['slashing']::text[],    array['ver']::text[],                     1,   10, null, null, null, null,  15, 'gp',  3),
  ('maul',        null, 'Maul',        'weapon', 'maul',        'martialM', 2,  6, array['bludgeoning']::text[], array['hvy','two']::text[],            null, null, null, null, null, null,  10, 'gp', 10),
  ('morningstar', null, 'Morningstar', 'weapon', 'morningstar', 'martialM', 1,  8, array['piercing']::text[],    '{}',                                  null, null, null, null, null, null,  15, 'gp',  4),
  ('pike',        null, 'Pike',        'weapon', 'pike',        'martialM', 1, 10, array['piercing']::text[],    array['hvy','rch','two']::text[],      null, null,   16, null, null, null,   5, 'gp', 18),
  ('rapier',      null, 'Rapier',      'weapon', 'rapier',      'martialM', 1,  8, array['piercing']::text[],    array['fin']::text[],                  null, null, null, null, null, null,  25, 'gp',  2),
  ('shortsword',  null, 'Shortsword',  'weapon', 'shortsword',  'martialM', 1,  6, array['piercing']::text[],    array['fin','lgt']::text[],            null, null, null, null, null, null,  10, 'gp',  2),
  ('trident',     null, 'Trident',     'weapon', 'trident',     'martialM', 1,  6, array['piercing']::text[],    array['thr','ver']::text[],               1,    8, null,   20,   60, null,   5, 'gp',  4),
  ('war_pick',    null, 'War Pick',    'weapon', 'warpick',     'martialM', 1,  8, array['piercing']::text[],    '{}',                                  null, null, null, null, null, null,   5, 'gp',  2),
  ('warhammer',   null, 'Warhammer',   'weapon', 'warhammer',   'martialM', 1,  8, array['bludgeoning']::text[], array['ver']::text[],                     1,   10, null, null, null, null,  15, 'gp',  2),
  ('whip',        null, 'Whip',        'weapon', 'whip',        'martialM', 1,  4, array['slashing']::text[],    array['fin','rch']::text[],            null, null,   16, null, null, null,   2, 'gp',  3),

  -- martial ranged
  ('crossbow_hand', null, 'Crossbow, Hand', 'weapon', 'handcrossbow', 'martialR', 1, 6, array['piercing']::text[], array['amm','lgt','lod']::text[], null, null, null,  30, 120, null, 75, 'gp', 3),
  ('longbow',       null, 'Longbow',        'weapon', 'longbow',      'martialR', 1, 8, array['piercing']::text[], array['amm','hvy','two']::text[], null, null, null, 150, 600, null, 50, 'gp', 2),
  -- No damage at all, and that is the row being correct rather than
  -- incomplete: a net restrains, and `spc` is where that goes once
  -- there is a condition to apply.
  ('net',           null, 'Net',            'weapon', 'net',          'martialR', null, null, '{}',            array['spc','thr']::text[],       null, null, null,   5,  15, null,  1, 'gp', 3);

-- ---------------------------------------------------------------------
-- Armour.
--
-- dex_cap null means no cap and that is what makes it light armour;
-- 0 is heavy, where Dex does not apply at all. equipment.rs already
-- reads it that way - this migration writes no new rule, it fills in
-- the table the rule was waiting for.
--
-- A shield is base_ac 2 and the engine ADDS it, which is why it is not
-- 12 of anything. It is also why check_one_armor lets a shield through
-- beside a breastplate: the rule is one suit, not one piece.
-- ---------------------------------------------------------------------

insert into public.items
  (key, game_id, name, kind, base_item,
   armor_category, base_ac, dex_cap, properties,
   rarity, price, denom, weight)
values
  ('padded',          null, 'Padded Armor',    'armor', 'padded',         'lgt', 11, null, array['stealthDisadvantage']::text[], null,    5, 'gp',  8),
  ('leather',         null, 'Leather Armor',   'armor', 'leather',        'lgt', 11, null, '{}',                                 null,   10, 'gp', 10),
  ('studded_leather', null, 'Studded Leather', 'armor', 'studdedleather', 'lgt', 12, null, '{}',                                 null,   45, 'gp', 13),

  ('hide',            null, 'Hide Armor',      'armor', 'hide',           'med', 12,    2, '{}',                                 null,   10, 'gp', 12),
  ('chain_shirt',     null, 'Chain Shirt',     'armor', 'chainshirt',     'med', 13,    2, '{}',                                 null,   50, 'gp', 20),
  ('breastplate',     null, 'Breastplate',     'armor', 'breastplate',    'med', 14,    2, '{}',                                 null,  400, 'gp', 20),
  ('half_plate',      null, 'Half Plate',      'armor', 'halfplate',      'med', 15,    2, array['stealthDisadvantage']::text[], null,  750, 'gp', 40),

  ('ring_mail',       null, 'Ring Mail',       'armor', 'ringmail',       'hvy', 14,    0, array['stealthDisadvantage']::text[], null,   30, 'gp', 40),
  ('chain_mail',      null, 'Chain Mail',      'armor', 'chainmail',      'hvy', 16,    0, array['stealthDisadvantage']::text[], null,   75, 'gp', 55),
  ('splint',          null, 'Splint Armor',    'armor', 'splint',         'hvy', 17,    0, array['stealthDisadvantage']::text[], null,  200, 'gp', 60),
  ('plate',           null, 'Plate Armor',     'armor', 'plate',          'hvy', 18,    0, array['stealthDisadvantage']::text[], null, 1500, 'gp', 65),

  ('shield',          null, 'Shield',          'armor', 'shield',         'shl',  2, null, '{}',                                 null,   10, 'gp',  6);
