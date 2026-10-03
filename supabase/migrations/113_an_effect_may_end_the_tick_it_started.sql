-- 113. AN EFFECT MAY END THE TICK IT STARTED.
--
-- 094 wrote `expires_at > started_at` to stop a zero-length effect
-- being created. It also stopped one being ENDED in the tick it began,
-- which is a normal thing: cast Bless, then cast Spirit Guardians on
-- the same turn, and 5e's one-concentration rule ends Bless at the
-- moment it started. `end_one` sets expires_at to now, now equals
-- started_at, and the whole write fails.
--
-- Found by probing the concentration path before shipping it. It would
-- have met Dave as a crash the first time a cleric changed their mind
-- in a single turn.
--
-- THE TWO RULES ARE DIFFERENT AND ONLY ONE BELONGS HERE. "Do not
-- create an effect that lasts no time" is about CREATION and already
-- lives in `effects::ends_at`, which returns None for anything zero or
-- negative and is tested. What a constraint should enforce is that
-- time does not run backwards.

alter table effects drop constraint if exists effects_ends_after_it_starts;
alter table effects add constraint effects_does_not_end_before_it_starts
  check (expires_at is null or expires_at >= started_at);
