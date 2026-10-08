-- 168. THE LAST TWO COLUMNS THE RESCUED PAIR NEVER HAD.
--
-- RECOVERED FROM THE DATABASE, NOT FROM THE LAPTOP THAT WROTE IT.
-- This migration was applied on 2026-10-07 at 21:46 UTC and its file
-- was never pushed. Supabase keeps the statements and strips the
-- comment header, so the SQL below is EXACTLY what ran - MD5-verified
-- against `supabase_migrations.schema_migrations` - and everything
-- above this line was written afterwards by somebody who was not there.
--
-- THE ORIGINAL HEADER IS STILL ON THE LAPTOP and is better than this
-- one. Overwrite this file from there; the SQL will match.
--
-- WHAT IT DOES, read off the statements: Fireball and Aura of Life are
-- the pair 105 rescued off one character's sheet rather than writing
-- from a list, and 159 found they had no `casting_time` at all. These
-- are the rest of what that pair never had - components, material,
-- range, and a duration for the aura.

update public.spells
   set components = array['v','s','m'],
       material   = 'a tiny ball of bat guano and sulphur',
       range      = '150 feet'
 where game_id is null
   and key = 'sp_fireball';

update public.spells
   set components = array['v'],
       range      = 'Self (30-foot radius)',
       duration   = 'Up to 10 minutes'
 where game_id is null
   and key = 'sp_auraoflife';
