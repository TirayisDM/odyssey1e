-- 093. WHAT RUNS OUT, AND WHAT BRINGS IT BACK.
--
-- `uses` and `recharge` for the base class features that have them.
-- Everything not listed here keeps NULL on both, which is the honest
-- answer for Evasion, Danger Sense and the forty other features that
-- are simply always true.
--
-- THE EXPRESSIONS ARE uses.rs's four forms. Bands read downward -
-- `1@1,2@17` is once from level 1 and twice from 17 - and `level` is
-- always THE CLASS'S level, which is why a Fighter 4 / Bard 1 gets one
-- Action Surge rather than the two a level 5 might suggest.
--
-- POOLS ARE DELIBERATELY ABSENT. Lay on Hands is five hit points per
-- level and Font of Magic is a pile of points: both are a MAGNITUDE
-- rather than a count of uses, and spending three of one is not the
-- same shape as using a feature once. They stay NULL until there is a
-- resource model, and the sheet shows them as text.

update class_features set uses = u.uses, recharge = u.recharge
from (values
  -- ---- short rest, the workhorses ----
  ('barbarian','rage',                  '2@1,3@3,4@6,5@12,6@17','long'),
  ('bard',     'bardic_inspiration',    'cha_mod',              'long'),
  ('cleric',   'channel_divinity',      '1@2,2@6,3@18',         'short'),
  ('druid',    'wild_shape',            '2',                    'short'),
  ('fighter',  'second_wind',           '1',                    'short'),
  ('fighter',  'action_surge',          '1@2,2@17',             'short'),
  ('fighter',  'indomitable',           '1@9,2@13,3@17',        'long'),
  ('monk',     'ki',                    'level',                'short'),
  ('monk',     'deflect_missiles',      '1',                    'short'),
  ('monk',     'slow_fall',             '1',                    'short'),
  ('monk',     'stillness_of_mind',     '1',                    'short'),
  ('monk',     'perfect_self',          '1',                    'short'),
  ('paladin',  'cleansing_touch',       'cha_mod',              'long'),
  ('paladin',  'divine_sense',          'cha_mod',              'long'),
  ('ranger',   'primeval_awareness',    '1',                    'long'),
  ('ranger',   'hide_in_plain_sight',   '1',                    'short'),
  ('rogue',    'stroke_of_luck',        '1',                    'short'),
  ('sorcerer', 'sorcerous_restoration', '1',                    'short'),
  ('warlock',  'mystic_arcanum',        '1@11,2@13,3@15,4@17',  'long'),
  ('warlock',  'eldritch_master',       '1',                    'long'),
  ('wizard',   'arcane_recovery',       '1',                    'day'),
  ('cleric',   'divine_intervention',   '1',                    'long'),
  ('wizard',   'signature_spells',      '2',                    'short')
) as u(class_key, key, uses, recharge)
where class_features.class_key = u.class_key
  and class_features.key = u.key
  and class_features.game_id is null;
