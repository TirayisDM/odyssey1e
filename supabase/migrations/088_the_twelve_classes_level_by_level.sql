-- 088. THE TWELVE CLASSES, LEVEL BY LEVEL.
--
-- Base class features for all twelve PHB classes. No subclasses and no
-- feats, by instruction: feats are a PHB OPTIONAL rule and the default
-- is ability scores only, and subclass features are the next pass.
--
-- THE TEXT IS OURS, NOT THE BOOK'S. Every summary here is written from
-- scratch to say what a feature DOES, in one line, so a player at the
-- table does not have to look it up. It is not the PHB's wording and is
-- not meant to replace owning the PHB - several of these features have
-- conditions and exceptions this does not carry.
--
-- ABILITY SCORE IMPROVEMENTS are 2 picks of +1, which is exactly 5e's
-- "increase one score by 2, or two scores by 1" without needing two
-- shapes to say it. Every class takes one at 4, 8, 12, 16 and 19; the
-- Fighter also at 6 and 14, and the Rogue also at 10.
--
-- THE SUBCLASS CHOICE POINTS ARE SEEDED AND LEFT UNCHOOSABLE -
-- `choose_from` NULL - so a sheet says "you pick a path at 3" without
-- offering a dropdown with nothing in it. They become choices when
-- there are archetypes to put in them.
--
-- EXTRA ATTACK APPEARS HERE AND IS ALSO `classes.extra_attack_levels`.
-- That is not a duplicated rule: the column is what the ENGINE reads to
-- decide how many swings an Attack action buys, and this row is what
-- the SHEET prints so a player can see the feature they gained. The
-- column stays the only thing anything calculates from.

-- Correcting 087's own table comment while we are here: the lock on a
-- choice is enforced by the RLS policies - an owner may insert and only
-- a DM may update or delete - rather than by the command, which is what
-- that comment said. The policy is the better place and is where it
-- actually ended up.
comment on table character_choices is
  'What a character chose where a feature made them choose. The only part of a feature that is not derived from the catalogue and their level, and so the only part stored. LOCKED ONCE SET by the policies rather than by a command: the owner may insert, and only the DM may update or delete - which is 5e, where retraining is at the DM''s say-so.';

insert into class_features (class_key, level, key, name, text, choose_from, picks) values

-- ============================ BARBARIAN ============================
('barbarian',1,'rage','Rage','Enter a rage as a bonus action for advantage on Strength checks and saves, bonus melee damage, and resistance to bludgeoning, piercing and slashing. A limited number per long rest.',null,1),
('barbarian',1,'unarmored_defense','Unarmored Defense','With no armour worn, your AC is 10 + DEX + CON. A shield still counts.',null,1),
('barbarian',2,'reckless_attack','Reckless Attack','Attack with advantage on your turn, and give everyone advantage against you until your next.',null,1),
('barbarian',2,'danger_sense','Danger Sense','Advantage on Dexterity saves against effects you can see.',null,1),
('barbarian',3,'primal_path','Primal Path','Choose a path. Its own features arrive at 3, 6, 10 and 14.',null,1),
('barbarian',5,'extra_attack','Extra Attack','Attack twice when you take the Attack action.',null,1),
('barbarian',5,'fast_movement','Fast Movement','Your speed rises by 10 feet while you are not in heavy armour.',null,1),
('barbarian',7,'feral_instinct','Feral Instinct','Advantage on initiative, and you may rage and act even when surprised.',null,1),
('barbarian',9,'brutal_critical','Brutal Critical','Roll one extra damage die on a melee critical. A second at 13, a third at 17.',null,1),
('barbarian',11,'relentless_rage','Relentless Rage','Dropping to 0 while raging, make a Constitution save to fall to 1 instead. The DC climbs each time.',null,1),
('barbarian',15,'persistent_rage','Persistent Rage','Your rage ends only when you choose, or when you fall unconscious.',null,1),
('barbarian',18,'indomitable_might','Indomitable Might','A Strength check that rolls lower than your Strength score counts as your Strength score.',null,1),
('barbarian',20,'primal_champion','Primal Champion','Strength and Constitution each rise by 4, to a maximum of 24.',null,1),

-- ============================== BARD ===============================
('bard',1,'spellcasting','Spellcasting','You cast bard spells, using Charisma. Known spells rather than prepared.',null,1),
('bard',1,'bardic_inspiration','Bardic Inspiration','As a bonus action, give an ally a die they can add to one check, attack or save. It does not stack with itself.',null,1),
('bard',2,'jack_of_all_trades','Jack of All Trades','Add half your proficiency bonus to any ability check that does not already include it.',null,1),
('bard',2,'song_of_rest','Song of Rest','Allies who spend hit dice on a short rest regain extra hit points.',null,1),
('bard',3,'bard_college','Bard College','Choose a college. Its own features arrive at 3, 6 and 14.',null,1),
('bard',3,'expertise_1','Expertise','Double your proficiency bonus on two skills you are proficient with.','skill',2),
('bard',5,'font_of_inspiration','Font of Inspiration','Your Bardic Inspiration comes back on a short rest as well as a long one.',null,1),
('bard',6,'countercharm','Countercharm','Allies who can hear you get advantage on saves against being frightened or charmed.',null,1),
('bard',10,'expertise_2','Expertise','Double your proficiency bonus on two more skills.','skill',2),
('bard',10,'magical_secrets','Magical Secrets','Learn two spells from any class. Two more at 14, and two more at 18.',null,1),
('bard',20,'superior_inspiration','Superior Inspiration','Regain a use of Bardic Inspiration when you roll initiative with none left.',null,1),

-- ============================= CLERIC ==============================
('cleric',1,'spellcasting','Spellcasting','You cast cleric spells, using Wisdom. Prepared from the whole list each day.',null,1),
('cleric',1,'divine_domain','Divine Domain','Choose a domain. Its own features arrive at 1, 2, 6, 8 and 17.',null,1),
('cleric',2,'channel_divinity','Channel Divinity','Turn Undead, and whatever else your domain grants. Once per rest, twice at 6 and three times at 18.',null,1),
('cleric',5,'destroy_undead','Destroy Undead','Weak undead that fail your Turn are destroyed outright. The threshold rises at 8, 11, 14 and 17.',null,1),
('cleric',10,'divine_intervention','Divine Intervention','Call on your deity for aid. The chance of an answer is your cleric level, and it is automatic at 20.',null,1),

-- ============================== DRUID ==============================
('druid',1,'druidic','Druidic','You speak Druidic, and can leave hidden messages in it that others find only by chance.',null,1),
('druid',1,'spellcasting','Spellcasting','You cast druid spells, using Wisdom. Prepared from the whole list each day.',null,1),
('druid',2,'wild_shape','Wild Shape','Take the form of a beast you have seen, twice per rest. What you may become improves with level.',null,1),
('druid',2,'druid_circle','Druid Circle','Choose a circle. Its own features arrive at 2, 6, 10 and 14.',null,1),
('druid',18,'timeless_body','Timeless Body','You age at a tenth the usual rate.',null,1),
('druid',18,'beast_spells','Beast Spells','You can cast in Wild Shape without needing hands or a voice.',null,1),
('druid',20,'archdruid','Archdruid','Wild Shape without limit, and ignore the voice and gesture a spell would need.',null,1),

-- ============================= FIGHTER =============================
('fighter',1,'fighting_style','Fighting Style','Choose a style: archery, defense, duelling, great weapon fighting, protection or two-weapon fighting.','fighting_style',1),
('fighter',1,'second_wind','Second Wind','As a bonus action, regain 1d10 + your fighter level in hit points. Once per rest.',null,1),
('fighter',2,'action_surge','Action Surge','Take one extra action on your turn. Once per rest, twice from 17.',null,1),
('fighter',3,'martial_archetype','Martial Archetype','Choose an archetype. Its own features arrive at 3, 7, 10, 15 and 18.',null,1),
('fighter',5,'extra_attack','Extra Attack','Attack twice when you take the Attack action. Three times at 11, four at 20.',null,1),
('fighter',9,'indomitable','Indomitable','Reroll a failed saving throw. Once per long rest, twice from 13 and three times from 17.',null,1),

-- ============================== MONK ===============================
('monk',1,'unarmored_defense','Unarmored Defense','With no armour and no shield, your AC is 10 + DEX + WIS.',null,1),
('monk',1,'martial_arts','Martial Arts','Use Dexterity for unarmed strikes and monk weapons, and strike again as a bonus action when you attack.',null,1),
('monk',2,'ki','Ki','Spend ki points on Flurry of Blows, Patient Defense and Step of the Wind. They come back on a short rest.',null,1),
('monk',2,'unarmored_movement','Unarmored Movement','Speed rises by 10 feet without armour or a shield, and keeps rising every few levels.',null,1),
('monk',3,'monastic_tradition','Monastic Tradition','Choose a tradition. Its own features arrive at 3, 6, 11 and 17.',null,1),
('monk',3,'deflect_missiles','Deflect Missiles','Use your reaction to reduce ranged weapon damage, and throw it back if you reduce it to nothing.',null,1),
('monk',4,'slow_fall','Slow Fall','Use your reaction to cut falling damage by five times your monk level.',null,1),
('monk',5,'extra_attack','Extra Attack','Attack twice when you take the Attack action.',null,1),
('monk',5,'stunning_strike','Stunning Strike','Spend a ki point on a hit to stun the target until the end of your next turn unless it saves.',null,1),
('monk',6,'ki_empowered_strikes','Ki-Empowered Strikes','Your unarmed strikes count as magical.',null,1),
('monk',7,'evasion','Evasion','Half damage becomes none, and full becomes half, on a Dexterity save.',null,1),
('monk',7,'stillness_of_mind','Stillness of Mind','End one effect charming or frightening you, as an action.',null,1),
('monk',10,'purity_of_body','Purity of Body','Immune to disease and poison.',null,1),
('monk',13,'tongue_of_sun_and_moon','Tongue of the Sun and Moon','You understand every spoken language, and anyone who can hear you understands you.',null,1),
('monk',14,'diamond_soul','Diamond Soul','Proficient in all saving throws, and you may spend a ki point to reroll a failure.',null,1),
('monk',15,'timeless_body','Timeless Body','You no longer age and cannot be aged by magic.',null,1),
('monk',18,'empty_body','Empty Body','Spend ki to turn invisible and resist all damage but force, or to cast astral projection.',null,1),
('monk',20,'perfect_self','Perfect Self','Regain 4 ki points when you roll initiative with none left.',null,1),

-- ============================= PALADIN =============================
('paladin',1,'divine_sense','Divine Sense','Sense celestials, fiends and undead nearby, and ground consecrated or desecrated.',null,1),
('paladin',1,'lay_on_hands','Lay on Hands','A pool of healing equal to five times your paladin level, spent by touch. It also cures disease and poison.',null,1),
('paladin',2,'fighting_style','Fighting Style','Choose a style: defense, duelling, great weapon fighting or protection.','fighting_style',1),
('paladin',2,'spellcasting','Spellcasting','You cast paladin spells, using Charisma. Prepared from the whole list each day.',null,1),
('paladin',2,'divine_smite','Divine Smite','Spend a spell slot on a melee hit for extra radiant damage, more against undead and fiends.',null,1),
('paladin',3,'divine_health','Divine Health','You cannot be diseased.',null,1),
('paladin',3,'sacred_oath','Sacred Oath','Choose an oath. Its own features arrive at 3, 7, 15 and 20.',null,1),
('paladin',5,'extra_attack','Extra Attack','Attack twice when you take the Attack action.',null,1),
('paladin',6,'aura_of_protection','Aura of Protection','You and allies nearby add your Charisma modifier to every saving throw.',null,1),
('paladin',10,'aura_of_courage','Aura of Courage','You and allies nearby cannot be frightened.',null,1),
('paladin',11,'improved_divine_smite','Improved Divine Smite','Every melee weapon hit does extra radiant damage, slot or no slot.',null,1),
('paladin',14,'cleansing_touch','Cleansing Touch','End a spell on yourself or someone you touch, a few times per long rest.',null,1),

-- ============================= RANGER ==============================
('ranger',1,'favored_enemy','Favored Enemy','Advantage on tracking and recalling lore about a chosen kind of creature, and a language they speak.',null,1),
('ranger',1,'natural_explorer','Natural Explorer','In a chosen terrain your group moves and forages far more effectively and cannot be lost by ordinary means.',null,1),
('ranger',2,'fighting_style','Fighting Style','Choose a style: archery, defense, duelling or two-weapon fighting.','fighting_style',1),
('ranger',2,'spellcasting','Spellcasting','You cast ranger spells, using Wisdom. Known spells rather than prepared.',null,1),
('ranger',3,'ranger_archetype','Ranger Archetype','Choose an archetype. Its own features arrive at 3, 7, 11 and 15.',null,1),
('ranger',3,'primeval_awareness','Primeval Awareness','Spend a spell slot to sense whether certain creature types are within miles of you.',null,1),
('ranger',5,'extra_attack','Extra Attack','Attack twice when you take the Attack action.',null,1),
('ranger',8,'lands_stride','Land''s Stride','Difficult terrain costs you nothing, and non-magical plants do not slow or harm you.',null,1),
('ranger',10,'hide_in_plain_sight','Hide in Plain Sight','Spend a minute camouflaging yourself for a large bonus to hiding while you stay still.',null,1),
('ranger',14,'vanish','Vanish','Hide as a bonus action, and you cannot be tracked by non-magical means.',null,1),
('ranger',18,'feral_senses','Feral Senses','Fight unseen creatures without penalty, and sense invisible ones nearby.',null,1),
('ranger',20,'foe_slayer','Foe Slayer','Once a turn, add your Wisdom modifier to an attack or damage roll against a favoured enemy.',null,1),

-- ============================== ROGUE ==============================
('rogue',1,'expertise_1','Expertise','Double your proficiency bonus on two skills you are proficient with, or one skill and thieves'' tools.','skill',2),
('rogue',1,'sneak_attack','Sneak Attack','Once a turn, extra damage on a finesse or ranged hit when you have advantage or an ally is adjacent. It scales with level.',null,1),
('rogue',1,'thieves_cant','Thieves'' Cant','A secret mix of dialect and sign that hides a message inside ordinary conversation.',null,1),
('rogue',2,'cunning_action','Cunning Action','Dash, Disengage or Hide as a bonus action.',null,1),
('rogue',3,'roguish_archetype','Roguish Archetype','Choose an archetype. Its own features arrive at 3, 9, 13 and 17.',null,1),
('rogue',5,'uncanny_dodge','Uncanny Dodge','Use your reaction to halve the damage of an attack you can see.',null,1),
('rogue',6,'expertise_2','Expertise','Double your proficiency bonus on two more skills.','skill',2),
('rogue',7,'evasion','Evasion','Half damage becomes none, and full becomes half, on a Dexterity save.',null,1),
('rogue',11,'reliable_talent','Reliable Talent','On a skill you are proficient with, treat any d20 below 10 as a 10.',null,1),
('rogue',14,'blindsense','Blindsense','You know where hidden or invisible creatures are within 10 feet, if you can hear.',null,1),
('rogue',15,'slippery_mind','Slippery Mind','Proficiency in Wisdom saving throws.',null,1),
('rogue',18,'elusive','Elusive','No attack has advantage against you while you are conscious.',null,1),
('rogue',20,'stroke_of_luck','Stroke of Luck','Turn a miss into a hit, or a failed check into a 20. Once per rest.',null,1),

-- ============================ SORCERER =============================
('sorcerer',1,'spellcasting','Spellcasting','You cast sorcerer spells, using Charisma. A small number known, and they never change except at level-up.',null,1),
('sorcerer',1,'sorcerous_origin','Sorcerous Origin','Choose an origin. Its own features arrive at 1, 6, 14 and 18.',null,1),
('sorcerer',2,'font_of_magic','Font of Magic','Sorcery points, traded back and forth with spell slots.',null,1),
('sorcerer',3,'metamagic','Metamagic','Learn two ways to bend a spell - twinned, quickened, subtle and the rest. Another at 10 and at 17.',null,1),
('sorcerer',20,'sorcerous_restoration','Sorcerous Restoration','Regain 4 sorcery points on a short rest.',null,1),

-- ============================= WARLOCK =============================
('warlock',1,'otherworldly_patron','Otherworldly Patron','Choose a patron. Its own features arrive at 1, 6, 10 and 14.',null,1),
('warlock',1,'pact_magic','Pact Magic','Few slots, always at your highest level, and they come back on a short rest.',null,1),
('warlock',2,'eldritch_invocations','Eldritch Invocations','Learn two lasting alterations to your magic, and more as you level. They can be swapped at level-up.',null,1),
('warlock',3,'pact_boon','Pact Boon','Choose a blade, a chain or a tome.',null,1),
('warlock',11,'mystic_arcanum','Mystic Arcanum','A 6th-level spell, cast once per long rest. A 7th at 13, an 8th at 15, a 9th at 17.',null,1),
('warlock',20,'eldritch_master','Eldritch Master','Spend a minute entreating your patron to regain every spell slot. Once per long rest.',null,1),

-- ============================== WIZARD =============================
('wizard',1,'spellcasting','Spellcasting','You cast wizard spells, using Intelligence, prepared from a spellbook you add to as you go.',null,1),
('wizard',1,'arcane_recovery','Arcane Recovery','Recover spell slots on a short rest, once a day, up to half your wizard level.',null,1),
('wizard',2,'arcane_tradition','Arcane Tradition','Choose a school. Its own features arrive at 2, 6, 10 and 14.',null,1),
('wizard',18,'spell_mastery','Spell Mastery','Cast one 1st-level and one 2nd-level spell at will.',null,1),
('wizard',20,'signature_spells','Signature Spells','Two 3rd-level spells always prepared, each castable once per short rest without a slot.',null,1)

on conflict do nothing;

-- ---------------------------------------------------------------------
-- ABILITY SCORE IMPROVEMENTS
-- ---------------------------------------------------------------------
--
-- Every class at 4, 8, 12, 16 and 19; the Fighter also at 6 and 14, and
-- the Rogue also at 10. Generated rather than typed out seventy times,
-- because a list that long typed by hand is a list with a level missing
-- in it.
insert into class_features (class_key, level, key, name, text, choose_from, picks)
select c.key,
       lv,
       'asi_' || lv,
       'Ability Score Improvement',
       'Raise one ability score by 2, or two of them by 1. Nothing goes above 20.',
       'ability',
       2
  from classes c
  cross join lateral (
    select unnest(
      case c.key
        when 'fighter' then array[4,6,8,12,14,16,19]
        when 'rogue'   then array[4,8,10,12,16,19]
        else                array[4,8,12,16,19]
      end) as lv
  ) levels
 where c.game_id is null
on conflict do nothing;
