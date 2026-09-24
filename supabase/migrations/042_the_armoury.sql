-- =====================================================================
-- 042_the_armoury.sql
-- odyssey1e — the weapons the SRD leaves out
-- =====================================================================
--
-- 027 seeded the SRD tables, which are deliberately short: 5e collapses
-- a century of European polearms into "glaive, halberd, pike" and calls
-- it done. That is the right call for a rulebook wanting nine pages of
-- equipment and the wrong one for a table that wants a bardiche to feel
-- different from a voulge.
--
-- These twenty are LOW TECH and historical - iron, ash and leverage, no
-- clockwork and no alchemy. Every one existed, and most of them existed
-- because somebody needed to solve a specific problem: getting a rider
-- off a horse, opening maille, catching a blade before it reached you.
-- The technique lists in 043 are built on those problems, which is why
-- the weapons come first.
--
-- MECHANICALLY THEY ARE 5e WEAPONS, not a new system. Same classes,
-- same properties, same damage shapes - a bardiche is 2d4 slashing with
-- heavy, two-handed and reach, which is a real 5e statline and not an
-- invented one. What makes them distinct is what they DO, and that is
-- techniques rather than numbers.
--
-- 2d4 APPEARS HERE AND NOT IN THE SRD, deliberately. Two dice of four
-- average the same as 1d8 and cluster harder - fewer ones, fewer
-- eights - which is exactly right for a weapon whose whole point is
-- reliable leverage rather than a lucky edge. AD&D used it for most of
-- the polearm table and it is worth keeping.
--
-- RANGE_REACH IS IN INCHES following 008: 8 is five feet, 16 is a reach
-- weapon. That inconsistency with range_value in feet is 008's and this
-- is still not the migration that fixes it.
-- =====================================================================

insert into public.items
  (key, game_id, name, kind, base_item,
   weapon_class, damage_number, damage_denomination, damage_types, properties,
   versatile_number, versatile_denomination,
   range_reach, range_value, range_long,
   size, slots, rarity, price, denom, weight)
values
  -- ---------------------------------------------------------------
  -- POLEARMS. The famous AD&D table, and the reason it existed: a man
  -- on foot needs eight feet of leverage to argue with a man on a
  -- horse. All reach, all two-handed, all heavy.
  -- ---------------------------------------------------------------
  ('bardiche',      null, 'Bardiche',       'weapon', 'bardiche',     'martialM', 2,  4, array['slashing']::text[],    array['hvy','rch','two']::text[], null, null, 16, null, null, 'lg', 1, null, 12, 'gp', 12),
  ('voulge',        null, 'Voulge',         'weapon', 'voulge',       'martialM', 1, 10, array['slashing']::text[],    array['hvy','rch','two']::text[], null, null, 16, null, null, 'lg', 1, null, 10, 'gp', 12),
  ('guisarme',      null, 'Guisarme',       'weapon', 'guisarme',     'martialM', 2,  4, array['slashing']::text[],    array['hvy','rch','two']::text[], null, null, 16, null, null, 'lg', 1, null, 14, 'gp', 10),
  ('ranseur',       null, 'Ranseur',        'weapon', 'ranseur',      'martialM', 2,  4, array['piercing']::text[],    array['hvy','rch','two']::text[], null, null, 16, null, null, 'lg', 1, null, 14, 'gp', 10),
  ('bec_de_corbin', null, 'Bec de Corbin',  'weapon', 'becdecorbin',  'martialM', 1, 10, array['bludgeoning']::text[], array['hvy','rch','two']::text[], null, null, 16, null, null, 'lg', 1, null, 15, 'gp', 12),
  ('military_fork', null, 'Military Fork',  'weapon', 'militaryfork', 'martialM', 1,  8, array['piercing']::text[],    array['hvy','rch','two']::text[], null, null, 16, null, null, 'lg', 1, null,  8, 'gp',  9),
  ('war_scythe',    null, 'War Scythe',     'weapon', 'warscythe',    'martialM', 2,  4, array['slashing']::text[],    array['hvy','rch','two']::text[], null, null, 16, null, null, 'lg', 1, null,  9, 'gp', 10),

  -- ---------------------------------------------------------------
  -- SWORDS. Four answers to four different armours.
  -- ---------------------------------------------------------------
  ('falchion',      null, 'Falchion',       'weapon', 'falchion',     'martialM', 1,  8, array['slashing']::text[],    '{}',                             null, null, null, null, null, 'med', 1, null, 18, 'gp',  5),
  ('khopesh',       null, 'Khopesh',        'weapon', 'khopesh',      'martialM', 1,  8, array['slashing']::text[],    '{}',                             null, null, null, null, null, 'med', 1, null, 22, 'gp',  5),
  ('gladius',       null, 'Gladius',        'weapon', 'gladius',      'martialM', 1,  6, array['piercing']::text[],    array['lgt']::text[],             null, null, null, null, null, 'sm',  1, null, 12, 'gp',  3),
  -- Two-handed and PIERCING: a stiff spike of a blade, made for the
  -- gaps in maille rather than for cutting anything at all.
  ('estoc',         null, 'Estoc',          'weapon', 'estoc',        'martialM', 1, 10, array['piercing']::text[],    array['two']::text[],             null, null, null, null, null, 'lg',  1, null, 25, 'gp',  5),
  ('seax',          null, 'Seax',           'weapon', 'seax',         'simpleM',  1,  6, array['slashing']::text[],    array['lgt']::text[],             null, null, null, null, null, 'sm',  1, null,  4, 'gp',  2),

  -- ---------------------------------------------------------------
  -- AXES, PICKS AND HAMMERS. Concentrated force, in three shapes.
  -- ---------------------------------------------------------------
  ('francisca',      null, 'Francisca',       'weapon', 'francisca',     'simpleM',  1, 6, array['slashing']::text[],    array['lgt','thr']::text[], null, null, null, 20, 60, 'sm',  1, null,  6, 'gp', 2),
  ('bearded_axe',    null, 'Bearded Axe',     'weapon', 'beardedaxe',    'martialM', 1, 8, array['slashing']::text[],    array['ver']::text[],          1,   10, null, null, null, 'med', 1, null, 12, 'gp', 5),
  ('horsemans_pick', null, 'Horseman''s Pick','weapon', 'horsemanspick', 'martialM', 1, 6, array['piercing']::text[],    array['lgt']::text[],       null, null, null, null, null, 'sm',  1, null, 10, 'gp', 3),
  ('flanged_mace',   null, 'Flanged Mace',    'weapon', 'flangedmace',   'martialM', 1, 8, array['bludgeoning']::text[], '{}',                       null, null, null, null, null, 'med', 1, null, 14, 'gp', 5),

  -- ---------------------------------------------------------------
  -- HAFTED AND FLEXIBLE. A flail goes round a shield; a boar spear
  -- stops something that wants to reach you.
  -- ---------------------------------------------------------------
  ('war_flail',  null, 'War Flail',  'weapon', 'warflail',  'martialM', 1, 10, array['bludgeoning']::text[], array['hvy','two']::text[], null, null, null, null, null, 'lg',  1, null, 12, 'gp', 10),
  ('boar_spear', null, 'Boar Spear', 'weapon', 'boarspear', 'martialM', 1,  8, array['piercing']::text[],    array['rch','spc']::text[], null, null,   16, null, null, 'lg',  1, null,  5, 'gp',  6),

  -- ---------------------------------------------------------------
  -- THROWN AND SLUNG.
  -- ---------------------------------------------------------------
  ('bolas',       null, 'Bolas',       'weapon', 'bolas',      'simpleR', 1, 4, array['bludgeoning']::text[], array['spc','thr']::text[], null, null, null, 20,  60, 'sm',  1, null,  5, 'sp', 2),
  ('staff_sling', null, 'Staff Sling', 'weapon', 'staffsling', 'simpleR', 1, 6, array['bludgeoning']::text[], array['amm','two']::text[], null, null, null, 60, 240, 'med', 1, null,  2, 'sp', 3);
