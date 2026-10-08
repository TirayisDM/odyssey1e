-- 164. TWELVE SPELLS THAT DID NO DAMAGE.
--
-- 161, 162 and 163 added 97 wizard spells. Every damaging save spell
-- among them went in with `on_save` NULL, because I filled the columns
-- the SRD prints - level, school, range, duration, components, save
-- ability, dice - and `on_save` is not one of them. It is ours.
--
-- 158 GAVE NULL A MEANING: the dice are not save damage at all. Bane's
-- 1d4 is a penalty, Geas's 5d10 is a toll for disobeying, Bestow
-- Curse's 1d8 is one of four options. `spellcast::save_damage` returns
-- `None` for all three and the caller writes no event, because 013's
-- log records what changed.
--
-- So Lightning Bolt has been doing nothing. Cone of Cold, Wall of
-- Fire, Cloudkill, Burning Hands, Thunderwave, Shatter, Flaming
-- Sphere, Blight, Ice Storm, Acid Splash, Poison Spray - twelve
-- spells that roll dice, print a DC, and leave hit points alone.
--
-- 159 ALL OVER AGAIN, which is the part worth keeping. Fireball had no
-- casting time and read as `TooLong` - a bad answer that announced
-- itself the first time somebody reached for it. This is the quiet
-- version: the spell casts, the save is rolled, the card shows the
-- dice, and the absence is invisible unless you are watching hit
-- points. 158's NULL is right for the three spells it was written for
-- and silent for every spell seeded without it.
--
-- NOT A DATA MIGRATION. 129 and 132 are the record of a balance change
-- arriving as a side effect of one, so this is on its own: twelve
-- spells start doing damage today and nothing else changes.
--
-- THE THREE THAT STAY NULL are the three 158 named. Bane, Geas and
-- Bestow Curse are deliberate, not missed, and a sweep that fills
-- every NULL would break them.

update public.spells set on_save = 'half'
 where game_id is null
   and key in ('sp_burninghands','sp_thunderwave','sp_flamingsphere',
               'sp_shatter','sp_lightningbolt','sp_blight','sp_icestorm',
               'sp_walloffire','sp_cloudkill','sp_coneofcold');

-- The two cantrips take nothing on a success. 5e gives half damage to
-- levelled area spells and all-or-nothing to these two.
update public.spells set on_save = 'none'
 where game_id is null
   and key in ('sp_acidsplash','sp_poisonspray');
