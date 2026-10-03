-- 115. TWO GRANTS THAT WERE NOT TRUE.
--
-- 114 seeded five and two of them were wrong. Correcting them in their
-- own migration rather than editing 114, because a seed that was live
-- for ten minutes is still a thing that happened.
--
-- PROTECTION FROM EVIL AND GOOD DOES NOT ADD 1d4 TO ANYTHING. It gives
-- certain creature types disadvantage to hit you and stops them
-- charming, frightening or possessing you. That is advantage and
-- immunity, not a number, and I invented the d4.
--
-- RESISTANCE IS ONE SAVE, NOT EVERY SAVE. The book is explicit: "add
-- 1d4 to one saving throw of its choice", and it is spent when used. A
-- persistent grant would have given a minute of +1d4 on every save a
-- character made - several times better than the cantrip is.
--
-- GUIDANCE HAS THE SAME SHAPE and was already left empty for it: the
-- vocabulary has no "one check, once" and inventing one to carry two
-- cantrips would be the wrong place to put that rule. Both are tracked
-- as effects and move no number; the DM adds the d4 when it is spent.
--
-- WHAT STAYS IS WHAT IS GENUINELY PERSISTENT AND NUMERIC: Bless, Bane
-- and Shield of Faith. All three last a stated time, apply to every
-- qualifying roll in it, and say so in the book in exactly the shape
-- grants.rs already had.

update spells set grants = '[]'::jsonb
 where game_id is null
   and key in ('sp_protectionfromevilandgood', 'sp_resistance');
