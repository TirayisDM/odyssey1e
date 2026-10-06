-- 148. A DRAGON THAT CAN BREATHE.
--
-- 147's header listed what the bestiary still had no way to say, and
-- breath weapons were the first of it: "a dragon here bites, claws,
-- lashes and buffets; it does not breathe." 43 dragons and the thing
-- everybody at the table is actually afraid of was a sentence nobody had
-- written down.
--
-- IT FITS THE EXISTING VOCABULARY, which is Dave's point and turns out to
-- be right. A breath weapon is a natural weapon: an ITEM with its own
-- dice and its own damage type, equipped like a bite, rolled through the
-- same `attack::resolve` as everything else. No new mechanism.
--
-- ---------------------------------------------------------------------
-- ONE ITEM PER DRAGON, WHICH LOOKS WASTEFUL AND IS NOT
-- ---------------------------------------------------------------------
--
-- 41 items for 41 breaths, where five - one per element - would have been
-- tidier. The dice differ by COLOUR AND AGE: a red wyrmling breathes 7d6
-- and an ancient red 26d6, and a copper is not a brass.
--
-- A shared `breath_fire` carrying four techniques at four gates would
-- have been the obvious shape, and 133 is exactly why it cannot be: a
-- creature clears every gate, so a wyrmling equipped with that item
-- would reach the ancient dragon's breath. The gate is not available as
-- a discriminator any more, so the ITEM has to be the discriminator.
-- That is 133 working as intended rather than getting in the way.
--
-- ---------------------------------------------------------------------
-- IT IS AIMED, AND THE PROSE SAYS WHERE THE REAL RULE IS
-- ---------------------------------------------------------------------
--
-- 136 met this with the Shrieker and chose the same way: this engine
-- rolls to hit against an AC and has no save-DC path, so a breath is
-- modelled as an attack and the save is written out in the item's own
-- description - the shape, the DC and which save.
--
-- AND THE DM HAS SOMEWHERE TO PUT IT. `encounter_challenges` supplies a
-- DC and 011's own comment says "the roll path cannot tell them apart"
-- from an actor's AC. So a DM adds "Fire Breath" at DC 21, the table
-- rolls DEX saves against it, and this item supplies the damage. Both
-- halves exist; they are two clicks rather than one button, and the
-- description says so rather than leaving it to be discovered.
--
-- ---------------------------------------------------------------------
-- CON, AND NO MODIFIER ON THE DAMAGE
-- ---------------------------------------------------------------------
--
-- `properties` carries `con` - 134, the Shrieker's lesson, because a
-- breath is not a strength attack - and `nomod`, which is new in 148.
--
-- 5e puts an ability modifier on the damage of a weapon you SWING, since
-- the arm is doing the work, and puts none on a dragon's breath. The
-- engine had no way to say so and an ancient red's breath came back as
-- 26d6+9: a plausible number, correctly derived, and not the one in the
-- book. `attack::damage_mod` reads the property and nothing else does.
-- Nothing a person carries has it.
--
-- ---------------------------------------------------------------------
-- WHAT THE NUMBERS ARE, AND WHAT THE WORDS ARE
-- ---------------------------------------------------------------------
--
-- Every die, DC, cone and line below was read off the published SRD, the
-- same source 146 and 147 used. Every sentence of description is written
-- for this bestiary.
--
-- `simpleR` AND A RANGE, so the cone or line length lands in
-- `range_value` where the sheet already knows how to show a distance,
-- and so the mode is not melee. No `amm`, so nothing asks a dragon for
-- ammunition.
--
-- STILL NOT HERE: the second breath some metallic dragons have - the
-- sleep, the weakening, the paralysing gas - which are conditions rather
-- than damage and have no more home in this schema than a grapple does.
-- The damage breath is the one that decides fights.

insert into items
  (key, name, kind, weapon_class, damage_number, damage_denomination, damage_types,
   properties, content_tags, size, weight, slots, price, denom, range_value, description)
values
('breath_black_wyrmling','Acid Breath','weapon','simpleR',5,8,array['acid'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',15,'A 15-foot line. DC 11 DEX for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_black_young','Acid Breath','weapon','simpleR',11,8,array['acid'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',30,'A 30-foot line. DC 14 DEX for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_black_adult','Acid Breath','weapon','simpleR',12,8,array['acid'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',60,'A 60-foot line. DC 18 DEX for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_black_ancient','Acid Breath','weapon','simpleR',15,8,array['acid'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',90,'A 90-foot line. DC 22 DEX for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_blue_wyrmling','Lightning Breath','weapon','simpleR',4,10,array['lightning'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',30,'A 30-foot line. DC 12 DEX for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_blue_young','Lightning Breath','weapon','simpleR',10,10,array['lightning'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',60,'A 60-foot line. DC 16 DEX for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_blue_adult','Lightning Breath','weapon','simpleR',12,10,array['lightning'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',90,'A 90-foot line. DC 19 DEX for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_blue_ancient','Lightning Breath','weapon','simpleR',16,10,array['lightning'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',120,'A 120-foot line. DC 23 DEX for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_green_wyrmling','Poison Breath','weapon','simpleR',6,6,array['poison'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',15,'A 15-foot cone. DC 11 CON for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_green_young','Poison Breath','weapon','simpleR',12,6,array['poison'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',30,'A 30-foot cone. DC 14 CON for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_green_adult','Poison Breath','weapon','simpleR',16,6,array['poison'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',60,'A 60-foot cone. DC 18 CON for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_green_ancient','Poison Breath','weapon','simpleR',22,6,array['poison'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',90,'A 90-foot cone. DC 22 CON for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_red_wyrmling','Fire Breath','weapon','simpleR',7,6,array['fire'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',15,'A 15-foot cone. DC 13 DEX for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_red_young','Fire Breath','weapon','simpleR',16,6,array['fire'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',30,'A 30-foot cone. DC 17 DEX for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_red_adult','Fire Breath','weapon','simpleR',18,6,array['fire'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',60,'A 60-foot cone. DC 21 DEX for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_red_ancient','Fire Breath','weapon','simpleR',26,6,array['fire'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',90,'A 90-foot cone. DC 24 DEX for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_white_wyrmling','Cold Breath','weapon','simpleR',5,8,array['cold'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',15,'A 15-foot cone. DC 12 CON for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_white_young','Cold Breath','weapon','simpleR',10,8,array['cold'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',30,'A 30-foot cone. DC 15 CON for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_white_adult','Cold Breath','weapon','simpleR',12,8,array['cold'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',60,'A 60-foot cone. DC 19 CON for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_white_ancient','Cold Breath','weapon','simpleR',16,8,array['cold'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',90,'A 90-foot cone. DC 22 CON for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_brass_wyrmling','Fire Breath','weapon','simpleR',4,6,array['fire'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',20,'A 20-foot line. DC 11 DEX for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_brass_young','Fire Breath','weapon','simpleR',12,6,array['fire'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',40,'A 40-foot line. DC 14 DEX for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_brass_adult','Fire Breath','weapon','simpleR',13,6,array['fire'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',60,'A 60-foot line. DC 18 DEX for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_brass_ancient','Fire Breath','weapon','simpleR',16,6,array['fire'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',90,'A 90-foot line. DC 21 DEX for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_bronze_wyrmling','Lightning Breath','weapon','simpleR',3,10,array['lightning'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',40,'A 40-foot line. DC 12 DEX for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_bronze_young','Lightning Breath','weapon','simpleR',10,10,array['lightning'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',60,'A 60-foot line. DC 15 DEX for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_bronze_adult','Lightning Breath','weapon','simpleR',12,10,array['lightning'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',90,'A 90-foot line. DC 19 DEX for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_bronze_ancient','Lightning Breath','weapon','simpleR',16,10,array['lightning'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',120,'A 120-foot line. DC 23 DEX for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_copper_wyrmling','Acid Breath','weapon','simpleR',4,8,array['acid'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',20,'A 20-foot line. DC 11 DEX for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_copper_young','Acid Breath','weapon','simpleR',9,8,array['acid'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',40,'A 40-foot line. DC 14 DEX for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_copper_adult','Acid Breath','weapon','simpleR',12,8,array['acid'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',60,'A 60-foot line. DC 18 DEX for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_copper_ancient','Acid Breath','weapon','simpleR',14,8,array['acid'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',90,'A 90-foot line. DC 22 DEX for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_gold_wyrmling','Fire Breath','weapon','simpleR',4,10,array['fire'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',15,'A 15-foot cone. DC 13 DEX for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_gold_young','Fire Breath','weapon','simpleR',10,10,array['fire'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',30,'A 30-foot cone. DC 17 DEX for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_gold_adult','Fire Breath','weapon','simpleR',12,10,array['fire'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',60,'A 60-foot cone. DC 21 DEX for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_gold_ancient','Fire Breath','weapon','simpleR',13,10,array['fire'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',90,'A 90-foot cone. DC 24 DEX for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_silver_wyrmling','Cold Breath','weapon','simpleR',4,8,array['cold'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',15,'A 15-foot cone. DC 13 CON for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_silver_young','Cold Breath','weapon','simpleR',12,8,array['cold'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',30,'A 30-foot cone. DC 17 CON for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_silver_adult','Cold Breath','weapon','simpleR',13,8,array['cold'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',60,'A 60-foot cone. DC 20 CON for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_silver_ancient','Cold Breath','weapon','simpleR',15,8,array['cold'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',90,'A 90-foot cone. DC 24 CON for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.'),
('breath_steam','Steam Breath','weapon','simpleR',15,6,array['fire'],array['con','nomod'],array['natural'],'tiny',0,0,0,'cp',60,'A 60-foot cone. DC 18 CON for half - this engine cannot roll a save, so add the DC as a challenge and use this for the damage. Recharge 5-6.')
on conflict (key) where game_id is null do update set
  name = excluded.name, kind = excluded.kind, weapon_class = excluded.weapon_class,
  damage_number = excluded.damage_number,
  damage_denomination = excluded.damage_denomination,
  damage_types = excluded.damage_types, properties = excluded.properties,
  content_tags = excluded.content_tags, size = excluded.size,
  weight = excluded.weight, slots = excluded.slots, price = excluded.price,
  denom = excluded.denom, range_value = excluded.range_value,
  description = excluded.description;

-- AND INTO THE DRAGON THAT BREATHES IT. One each, equipped - 135's rule
-- is that a statblock with no kit is a creature that cannot act, and the
-- corollary is that a dragon whose breath is only in its prose is a
-- dragon that cannot breathe.

insert into npc_items (npc_key, item_key, quantity, equipped)
values
('black_dragon_wyrmling','breath_black_wyrmling',1,true),
('young_black_dragon','breath_black_young',1,true),
('adult_black_dragon','breath_black_adult',1,true),
('ancient_black_dragon','breath_black_ancient',1,true),
('blue_dragon_wyrmling','breath_blue_wyrmling',1,true),
('young_blue_dragon','breath_blue_young',1,true),
('adult_blue_dragon','breath_blue_adult',1,true),
('ancient_blue_dragon','breath_blue_ancient',1,true),
('green_dragon_wyrmling','breath_green_wyrmling',1,true),
('young_green_dragon','breath_green_young',1,true),
('adult_green_dragon','breath_green_adult',1,true),
('ancient_green_dragon','breath_green_ancient',1,true),
('red_dragon_wyrmling','breath_red_wyrmling',1,true),
('young_red_dragon','breath_red_young',1,true),
('adult_red_dragon','breath_red_adult',1,true),
('ancient_red_dragon','breath_red_ancient',1,true),
('white_dragon_wyrmling','breath_white_wyrmling',1,true),
('young_white_dragon','breath_white_young',1,true),
('adult_white_dragon','breath_white_adult',1,true),
('ancient_white_dragon','breath_white_ancient',1,true),
('brass_dragon_wyrmling','breath_brass_wyrmling',1,true),
('young_brass_dragon','breath_brass_young',1,true),
('adult_brass_dragon','breath_brass_adult',1,true),
('ancient_brass_dragon','breath_brass_ancient',1,true),
('bronze_dragon_wyrmling','breath_bronze_wyrmling',1,true),
('young_bronze_dragon','breath_bronze_young',1,true),
('adult_bronze_dragon','breath_bronze_adult',1,true),
('ancient_bronze_dragon','breath_bronze_ancient',1,true),
('copper_dragon_wyrmling','breath_copper_wyrmling',1,true),
('young_copper_dragon','breath_copper_young',1,true),
('adult_copper_dragon','breath_copper_adult',1,true),
('ancient_copper_dragon','breath_copper_ancient',1,true),
('gold_dragon_wyrmling','breath_gold_wyrmling',1,true),
('young_gold_dragon','breath_gold_young',1,true),
('adult_gold_dragon','breath_gold_adult',1,true),
('ancient_gold_dragon','breath_gold_ancient',1,true),
('silver_dragon_wyrmling','breath_silver_wyrmling',1,true),
('young_silver_dragon','breath_silver_young',1,true),
('adult_silver_dragon','breath_silver_adult',1,true),
('ancient_silver_dragon','breath_silver_ancient',1,true),
('dragon_turtle','breath_steam',1,true)
on conflict (npc_key, item_key) where game_id is null do update set
  quantity = excluded.quantity, equipped = excluded.equipped;
