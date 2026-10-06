-- 151. FIVE CREATURES THAT CAST.
--
-- 150 built the path; this is the first data down it. Three clerics and
-- two wizards, and nothing about any of them is creature-shaped: they
-- have a class, a level in it, and a list, which is what a player
-- character has.
--
--                   class       book CR   our `level`
--   Priest          cleric 5    2         5
--   Cult Fanatic    cleric 4    2         6
--   Acolyte         cleric 1    1/4       2
--   Mage            wizard 9    6         9
--   Lich            wizard 18   21        18
--
-- THE CLASS LEVEL IS NOT THE CHALLENGE RATING - 150's header argues it
-- and the middle column shows it. The third column is the catch: these
-- five are 130's creatures, whose `level` was hand-set on 127's scale
-- rather than from CR, and for three of them it happens to land on the
-- caster level. That is coincidence and not a rule. 147's creatures DO
-- carry CR in `level`, so reading `level` as a caster level would be
-- right here by luck and wrong for an Adult Gold Dragon.
--
-- ---------------------------------------------------------------------
-- WITHIN THE PREPARED COUNT, WHICH IS WORTH CHECKING AND I DID
-- ---------------------------------------------------------------------
--
-- `prepared_max` is class level + the casting modifier, and seeding a
-- creature past it would put it over its own limit the moment anybody
-- opened the tab - a warning fired by the seed rather than by play:
--
--   Priest        5 + WIS 16 (+3) = 8    seeded 8
--   Cult Fanatic  4 + WIS 13 (+1) = 5    seeded 5
--   Acolyte       1 + WIS 14 (+2) = 3    seeded 3
--   Mage          9 + INT 17 (+3) = 12   seeded 9
--   Lich         18 + INT 20 (+5) = 23   seeded 23, exactly full
--
-- Cantrips are counted separately and against `cantrips_known`, which is
-- why they are a different state rather than a short prepared list.
--
-- ---------------------------------------------------------------------
-- THE WIZARDS GET WHAT THE CATALOGUE HAS, WHICH IS MORE THAN EXPECTED
-- ---------------------------------------------------------------------
--
-- 101 to 105 seeded the cleric list, so I expected the two wizards to
-- come out threadbare. They do not: 34 of those spells are on the wizard
-- list as well, at every level from cantrip to 9th, because the two
-- lists overlap far more than the headline "the cleric list" suggests.
-- The Lich gets a full spread up to Gate and Astral Projection.
--
-- WHAT IS STILL MISSING IS THE WIZARD-ONLY SPELLS - no Magic Missile, no
-- Counterspell, no Wall of Force. A Mage casting Fireball and Banishment
-- is a real Mage; it is not the whole one. That is a data gap with a
-- known shape, and it is the same job 102-104 did for the cleric.
--
-- EVERY SPELL BELOW IS MARKED `prepared` rather than `book`. A monster's
-- statblock prints what it has ready and says nothing about what it
-- chose not to prepare, so its book and its prepared list are the same
-- set - which is what `npc_spells`' own comment says. The book state is
-- for a player wizard, who has one.

update public.npcs set class_key = 'cleric', class_level = 5  where game_id is null and key = 'priest';
update public.npcs set class_key = 'cleric', class_level = 4  where game_id is null and key = 'cult_fanatic';
update public.npcs set class_key = 'cleric', class_level = 1  where game_id is null and key = 'acolyte';
update public.npcs set class_key = 'wizard', class_level = 9  where game_id is null and key = 'mage';
update public.npcs set class_key = 'wizard', class_level = 18 where game_id is null and key = 'lich';

insert into public.npc_spells (npc_key, spell_key, state) values
-- the Priest
('priest','sp_light','cantrip'),('priest','sp_sacredflame','cantrip'),('priest','sp_thaumaturgy','cantrip'),
('priest','sp_bless','prepared'),('priest','sp_curewounds','prepared'),('priest','sp_guidingbolt','prepared'),
('priest','sp_sanctuary','prepared'),('priest','sp_lesserrestoration','prepared'),
('priest','sp_spiritualweapon','prepared'),('priest','sp_dispelmagic','prepared'),
('priest','sp_spiritguardians','prepared'),
-- the Cult Fanatic
('cult_fanatic','sp_light','cantrip'),('cult_fanatic','sp_sacredflame','cantrip'),
('cult_fanatic','sp_thaumaturgy','cantrip'),
('cult_fanatic','sp_command','prepared'),('cult_fanatic','sp_inflictwounds','prepared'),
('cult_fanatic','sp_shieldoffaith','prepared'),('cult_fanatic','sp_holdperson','prepared'),
('cult_fanatic','sp_spiritualweapon','prepared'),
-- the Acolyte
('acolyte','sp_light','cantrip'),('acolyte','sp_sacredflame','cantrip'),('acolyte','sp_thaumaturgy','cantrip'),
('acolyte','sp_bless','prepared'),('acolyte','sp_curewounds','prepared'),('acolyte','sp_sanctuary','prepared'),
-- the Mage
('mage','sp_light','cantrip'),('mage','sp_mending','cantrip'),
('mage','sp_detectmagic','prepared'),('mage','sp_blindnessdeafness','prepared'),
('mage','sp_holdperson','prepared'),('mage','sp_fireball','prepared'),
('mage','sp_dispelmagic','prepared'),('mage','sp_banishment','prepared'),
('mage','sp_stoneshape','prepared'),('mage','sp_scrying','prepared'),
('mage','sp_planarbinding','prepared'),
-- the Lich
('lich','sp_light','cantrip'),('lich','sp_mending','cantrip'),
('lich','sp_detectmagic','prepared'),('lich','sp_protectionfromevilandgood','prepared'),
('lich','sp_blindnessdeafness','prepared'),('lich','sp_holdperson','prepared'),
('lich','sp_animatedead','prepared'),('lich','sp_bestowcurse','prepared'),
('lich','sp_dispelmagic','prepared'),('lich','sp_fireball','prepared'),
('lich','sp_magiccircle','prepared'),('lich','sp_banishment','prepared'),
('lich','sp_locatecreature','prepared'),('lich','sp_geas','prepared'),
('lich','sp_planarbinding','prepared'),('lich','sp_scrying','prepared'),
('lich','sp_createundead','prepared'),('lich','sp_trueseeing','prepared'),
('lich','sp_etherealness','prepared'),('lich','sp_planeshift','prepared'),
('lich','sp_symbol','prepared'),('lich','sp_antimagicfield','prepared'),
('lich','sp_controlweather','prepared'),('lich','sp_astralprojection','prepared'),
('lich','sp_gate','prepared')
on conflict (npc_key, spell_key) where game_id is null do update set
  state = excluded.state;
