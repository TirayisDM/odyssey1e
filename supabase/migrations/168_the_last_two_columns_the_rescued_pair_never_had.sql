-- 168. THE LAST TWO COLUMNS THE RESCUED PAIR NEVER HAD.
--
-- A sweep for half-filled rows after finishing the wizard list turned up
-- exactly two spells with an EMPTY COMPONENTS ARRAY, and they are the
-- same two 159 found with no casting time: sp_fireball and
-- sp_auraoflife, the pair 105 rescued off ONE CHARACTER'S SHEET instead
-- of taking from a list.
--
-- 159 FIXED THE SYMPTOM IN FRONT OF IT. Dave cast Hold Person, saw
-- Fireball greyed out, and the casting time was filled in. The header
-- named the cause correctly - these two came from a sheet and nobody
-- filled the columns in - and then fixed one column, because that was
-- the one that had failed out loud. The rest of the row was never
-- checked.
--
-- That is the shape worth recording: A CORRECT DIAGNOSIS AND A PARTIAL
-- FIX. Knowing the cause is a class of problem is not the same as
-- looking for the rest of the class, and the thing that found these was
-- not reasoning, it was `cardinality(components) = 0` run over the
-- whole table.
--
-- Fireball has had no V, S or M since 105 - a spell cast silently,
-- motionlessly and out of nothing, which no caster could be prevented
-- from casting while gagged, bound or stripped of a focus. Aura of Life
-- is verbal only and had nothing either.
--
-- AND THE FORMATS GO WITH THEM. Fireball is the only row in the
-- catalogue that writes its range as "150 ft" rather than "150 feet",
-- and Aura of Life is the only one that writes "10 min" rather than
-- naming the concentration cap the way 166 and 167 do. Both are the
-- same sheet showing through, so they are in the same file.
--
-- STILL NOT FIXED, AND DELIBERATELY: Fireball's `guidance` names a
-- particular campaign's deity. It is our own writing, so it breaks
-- nothing 102-104 set down, but it is one game's flavour sitting in the
-- shared catalogue. Changing somebody's prose is not a correction.
--
-- REUNITED. The SQL below is the database's own record of what ran,
-- MD5-verified when 8f726b4 recovered these four files; the header
-- above it is the original, off the laptop that wrote them. They were
-- separated because 165-168 were APPLIED WITHOUT THEIR HEADERS - the
-- statement was pasted on its own - and Supabase stores the statement,
-- so the reasoning never left this machine. 161-164 went in whole and
-- never lost anything.
--
-- THE LESSON IS ABOUT THE APPLY, NOT THE RECOVERY: a migration applied
-- body-only is a migration whose header exists in exactly one place.

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
