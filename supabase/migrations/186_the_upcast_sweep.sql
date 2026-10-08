-- 186. THE UPCAST SWEEP.
--
-- 185 built `at_higher_dice` and filled two rows - Magic Missile and
-- Fireball, the two in play - and said the rest wanted checking against
-- the published list a level at a time. This is that.
--
-- 30 SPELLS GAIN DICE FROM A BIGGER SLOT. Every value below was derived
-- from the published list first and then compared with the rider this
-- project already carried. That order matters: 183 refused to populate
-- a rules column by reading our own prose, because a typo in a sentence
-- would become a damage bug. Deriving first and using the rider as a
-- CHECK is the opposite move - two sources that have to agree.
--
-- ---------------------------------------------------------------------
-- THEY DID NOT ALL AGREE, AND THE PATTERN IS EXACT
-- ---------------------------------------------------------------------
--
-- ELEVEN SPELLS SCALE IN THE BOOK AND SAID NOTHING ABOUT IT:
--
--   Cure Wounds, Healing Word, Mass Healing Word, Mass Cure Wounds,
--   Prayer of Healing, Guiding Bolt, Inflict Wounds, Spirit Guardians,
--   Flame Strike, Insect Plague, Glyph of Warding
--
-- Every one is a CLERIC spell from 102-104. Every spell 161-167 seeded
-- carries its upcast line, because by then the riders were being
-- written from the list deliberately. The cleric half of the catalogue
-- was written earlier and never recorded it - so a cleric upcasting
-- Cure Wounds has been getting 1d8 since 102, and nothing on the card
-- even claimed otherwise.
--
-- THE SWEEP FOUND THEM BECAUSE IT LOOKED AT BOTH SIDES. Filling only
-- the spells whose riders mention a slot would have reproduced the gap
-- exactly, and confirmed it.
--
-- ---------------------------------------------------------------------
-- A DANGLING REFERENCE, FOUND ON THE WAY
-- ---------------------------------------------------------------------
--
-- Cure Wounds reads "Cast higher for more: see Cure Wounds II / III."
-- THOSE SPELLS DO NOT EXIST. Nothing in the catalogue has ever been
-- called that. A cleric with a 3rd-level slot was told to look up two
-- rows that were never seeded. The column makes the promise real, so
-- the rider stops pointing at nothing.
--
-- ---------------------------------------------------------------------
-- WHAT IS DELIBERATELY LEFT NULL
-- ---------------------------------------------------------------------
--
-- MORE TARGETS, NOT BIGGER DICE. Bless (+1 target), Chain Lightning
-- (+1 arc). The dice do not move.
--
-- SCORCHING RAY GAINS A WHOLE RAY, and a ray is its own ATTACK ROLL.
-- `at_higher_dice` of 2d6 would turn three attack rolls into one bigger
-- one, which is a different spell - a miss should cost one ray and not
-- all of them.
--
-- SPIRITUAL WEAPON IS +1d8 PER *TWO* SLOT LEVELS, and this column is
-- per level. There is no value that says "every other level", so it
-- says nothing rather than doubling the rate. The only one of its kind
-- in the catalogue, and the rider carries it.
--
-- BANE, BESTOW CURSE AND GEAS are 158's three: dice that were never
-- cast damage. They upcast duration, which no column here holds.
--
-- AND MOST OF THE REST SIMPLY DO NOT SCALE - Blade Barrier, Harm, Fire
-- Storm, Sunburst, Prismatic Spray and the 9th-level spells, which have
-- nothing above them to be cast with.
--
-- ---------------------------------------------------------------------
-- WHERE THE COLUMN HOLDS ONE HALF OF A SPELL
-- ---------------------------------------------------------------------
--
-- Several of these deal two damages and `dice` holds one, by decisions
-- 163 and 166 already took and documented. The scale matches WHAT IS
-- STORED, which is the only thing it can honestly scale:
--
--   Ice Storm        2d8 bludgeoning stored; +1d8 is the bludgeoning
--   Wall of Ice      10d6 on appearing; +2d6 is that one
--   Arcane Hand      4d8 fist stored; +2d8 is the fist
--   Acid Arrow       4d4 initial stored; the later 2d4 also gains 1d4
--   Flame Strike     4d6 stored; the book lets you choose which half
--
-- The riders say the rest and the DM adds it, exactly as before.

-- ---- the riders already said these ----
update public.spells set at_higher_dice = '1d6' where game_id is null and key = 'sp_burninghands';
update public.spells set at_higher_dice = '1d8' where game_id is null and key = 'sp_thunderwave';
update public.spells set at_higher_dice = '1d4' where game_id is null and key = 'sp_acidarrow';
update public.spells set at_higher_dice = '1d6' where game_id is null and key = 'sp_flamingsphere';
update public.spells set at_higher_dice = '1d8' where game_id is null and key = 'sp_shatter';
update public.spells set at_higher_dice = '1d6' where game_id is null and key = 'sp_lightningbolt';
update public.spells set at_higher_dice = '1d6' where game_id is null and key = 'sp_vampirictouch';
update public.spells set at_higher_dice = '1d8' where game_id is null and key = 'sp_blight';
update public.spells set at_higher_dice = '1d8' where game_id is null and key = 'sp_icestorm';
update public.spells set at_higher_dice = '1d10' where game_id is null and key = 'sp_phantasmalkiller';
update public.spells set at_higher_dice = '1d8' where game_id is null and key = 'sp_walloffire';
update public.spells set at_higher_dice = '2d8' where game_id is null and key = 'sp_arcanehand';
update public.spells set at_higher_dice = '1d8' where game_id is null and key = 'sp_cloudkill';
update public.spells set at_higher_dice = '1d8' where game_id is null and key = 'sp_coneofcold';
update public.spells set at_higher_dice = '2d6' where game_id is null and key = 'sp_circleofdeath';
update public.spells set at_higher_dice = '3d6' where game_id is null and key = 'sp_disintegrate';
update public.spells set at_higher_dice = '1d6' where game_id is null and key = 'sp_freezingsphere';
update public.spells set at_higher_dice = '2d6' where game_id is null and key = 'sp_wallofice';
update public.spells set at_higher_dice = '1d6' where game_id is null and key = 'sp_delayedblastfireball';

-- ---- the eleven the riders never mentioned, all from 102-104 ----
update public.spells set at_higher_dice = '1d8' where game_id is null and key = 'sp_curewounds';
update public.spells set at_higher_dice = '1d4' where game_id is null and key = 'sp_healingword';
update public.spells set at_higher_dice = '1d4' where game_id is null and key = 'sp_masshealingword';
update public.spells set at_higher_dice = '1d8' where game_id is null and key = 'sp_masscurewounds';
update public.spells set at_higher_dice = '1d8' where game_id is null and key = 'sp_prayerofhealing';
update public.spells set at_higher_dice = '1d6' where game_id is null and key = 'sp_guidingbolt';
update public.spells set at_higher_dice = '1d10' where game_id is null and key = 'sp_inflictwounds';
update public.spells set at_higher_dice = '1d8' where game_id is null and key = 'sp_spiritguardians';
update public.spells set at_higher_dice = '1d6' where game_id is null and key = 'sp_flamestrike';
update public.spells set at_higher_dice = '1d10' where game_id is null and key = 'sp_insectplague';
update public.spells set at_higher_dice = '1d8' where game_id is null and key = 'sp_glyphofwarding';

-- ---- and the rider that pointed at spells nobody ever wrote ----
update public.spells
   set special_text = 'A bigger slot heals more: +1d8 per slot level above 1st.'
 where game_id is null and key = 'sp_curewounds';

-- ---- the one the column cannot say, said in the text instead ----
update public.spells
   set special_text = special_text ||
       ' UPCASTING IS EVERY OTHER LEVEL: +1d8 per TWO slot levels above 2nd, which is the only cadence of its kind here and the one `at_higher_dice` cannot hold - so it is not set and the DM adds it.'
 where game_id is null and key = 'sp_spiritualweapon'
   and special_text not like '%EVERY OTHER LEVEL%';
