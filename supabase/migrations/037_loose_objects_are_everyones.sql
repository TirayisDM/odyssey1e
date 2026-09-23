-- =====================================================================
-- 037_loose_objects_are_everyones.sql
-- odyssey1e — finishing what 031 started and 033 half-finished
-- =====================================================================
--
-- 031 wrote three policies on `objects` from one template: the DM, or
-- the owner of whoever is holding it. That was right when the only
-- alternative to being held was being nowhere.
--
-- 033 gave things somewhere to lie and widened ONE of the three. The
-- UPDATE policy learned that a loose object - holder NULL, or a holder
-- that is a location - is any member's to move, which is what lets a
-- party pick a torch up off the floor. INSERT and DELETE were left on
-- the old template, and nothing noticed because the common paths do not
-- use them.
--
-- ---------------------------------------------------------------------
-- WHAT THE TWO GAPS ACTUALLY COST
--
-- A PLAYER CANNOT PART-DROP INTO A ROOM. `move_into` inserts a new row
-- when it splits a stack with nothing at the destination to merge into,
-- and that row's holder is the floor. `holder_character` of a floor is
-- NULL, so the INSERT policy's owner branch cannot match and the write
-- is refused outright - 42501, verified.
--
-- AND PICKING UP A LOOSE STACK DUPLICATED IT. This is the bad one.
-- `move_into` merges by ADDING to the destination row and then deleting
-- the source. The add is an UPDATE on the taker's own row, which 033
-- allows. The delete is of the LOOSE row, which the old policy refuses -
-- and a DELETE refused by row-level security does not raise. It matches
-- nothing and reports success.
--
-- So the taker gained the quantity, the floor kept the stack, and the
-- object existed twice. Nothing in the app could see it: PostgREST
-- answers 204, and 016 already wrote this lesson down once about an
-- UPDATE. A policy that hides a row does not refuse a write to it, it
-- pretends the row was not there.
--
-- ---------------------------------------------------------------------
-- WHY THE SAME CLAUSE RATHER THAN A NEW RULE
--
-- All three are now one sentence: the DM, the owner of whoever is
-- holding it, or any member when nobody is. Written three times because
-- Postgres wants a policy per verb - but it is ONE rule, and the three
-- drifting apart is exactly what happened here.
--
-- A helper would be the obvious tidy-up and is deliberately not done:
-- `holder_character` and `holder_is_a_location` are already the shared
-- parts, and wrapping the whole predicate would hide which verb it
-- applies to at the one moment somebody is reading these to find out.
-- =====================================================================

drop policy if exists "objects: holder or dm writes"  on public.objects;
drop policy if exists "objects: holder or dm deletes" on public.objects;

create policy "objects: holder or dm writes"
  on public.objects for insert
  with check (
    public.is_game_dm(game_id)
    or exists (select 1 from public.characters c
                where c.id = public.holder_character(holder_id)
                  and c.owner_uid = auth.uid())
    -- Loose, or lying in a place: any member may put something there.
    -- The same clause 033 gave the UPDATE policy, and for the same
    -- reason - a party dropping loot into a room is not world-building.
    or ((holder_id is null or public.holder_is_a_location(holder_id))
        and public.is_game_member(game_id))
  );

create policy "objects: holder or dm deletes"
  on public.objects for delete
  using (
    public.is_game_dm(game_id)
    or exists (select 1 from public.characters c
                where c.id = public.holder_character(holder_id)
                  and c.owner_uid = auth.uid())
    or ((holder_id is null or public.holder_is_a_location(holder_id))
        and public.is_game_member(game_id))
  );
