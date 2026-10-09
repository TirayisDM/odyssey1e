-- 198. A DM CAN SEE WHO IS AT THE TABLE.
--
-- Dave: "fix the game_members read so the DM sees their players".
--
-- **`game_members` WAS NOT THE PROBLEM.** Its SELECT policy is
-- `is_game_member(game_id)`, which already shows any member the whole
-- roster of their own game, and a probe confirms it: with two players
-- added, the DM reads three membership rows and jec-game-1's row stays
-- correctly invisible.
--
-- THE REPORT THAT STARTED THIS WAS WRONG ABOUT WHY. 197's verification
-- noted "the DM sees only 1 game_members row though his game has two
-- members" and concluded the read was broken. The roster was never
-- checked - there are TWO GAMES, `Test Game 1` and `jec-game-1`, each
-- with exactly one member who is its own DM. One row was the right
-- answer. The mistake was reading a count and inferring a cause, which
-- is the same mistake 159 made and 168 wrote down.
--
-- ---------------------------------------------------------------------
-- WHAT IS ACTUALLY MISSING: THE NAMES
-- ---------------------------------------------------------------------
--
-- The same probe asked for the roster WITH DISPLAY NAMES and got back
-- one: `Dave (DM) (dm)`. Three membership rows, one resolvable name.
--
-- `profiles` has exactly three policies and all three say `id =
-- current_profile()`. **A profile is readable only by the person it
-- belongs to**, so a DM holding three `profile_id`s can look up none of
-- them but his own. A roster built on this is a list of UUIDs.
--
-- That is the fault, and it is in `profiles`, not `game_members`.
--
-- ---------------------------------------------------------------------
-- THE RULE: SHARING A TABLE IS WHAT ENTITLES YOU TO A NAME
-- ---------------------------------------------------------------------
--
-- Not "the DM may read players" - both directions are needed. A player
-- has to be able to name the DM and the others at the table, because an
-- initiative strip that says who is up has to say it with a name. So
-- the predicate is symmetric: two people who share a game can read each
-- other's profile.
--
-- WHAT THAT EXPOSES is `display_name` and `created_at` to people you
-- are playing with, and to nobody else. It is the minimum a roster
-- needs and it does not widen with the number of games - a profile in
-- no game of yours stays unreadable.
--
-- `profiles: read own` STAYS. It is not made redundant: you must be
-- able to read your own profile before you have joined anything, which
-- is the sign-up path and what `me` depends on. Two permissive SELECT
-- policies OR together.

-- A SECURITY DEFINER HELPER, FOR THE REASON `is_game_member` IS ONE
-- (001). Written inline the policy would read `game_members` with RLS
-- applied, which means evaluating `game_members`'s own policy - and
-- that calls `is_game_member`, a definer function, so it would work but
-- would do the membership lookup twice per row. This does it once,
-- against `game_members_profile_idx`.
--
-- IT IS ALSO WHY THIS CANNOT RECURSE. The helper bypasses RLS, so
-- nothing it reads consults a policy, and in particular nothing it
-- reads consults `profiles`.
create or replace function public.shares_a_game(p_profile uuid)
returns boolean
language sql
stable
security definer
set search_path to ''
as $fn$
  select exists (
    select 1
      from public.game_members mine
      join public.game_members theirs on theirs.game_id = mine.game_id
     where mine.profile_id = public.current_profile()
       and theirs.profile_id = p_profile
  );
$fn$;

comment on function public.shares_a_game(uuid) is
  '198. Whether the current person and p_profile sit at the same table. '
  'The entitlement behind reading a fellow member''s display name.';

grant execute on function public.shares_a_game(uuid) to authenticated;

create policy "profiles: read a fellow member"
  on public.profiles for select to authenticated
  using (public.shares_a_game(id));

-- 195's check, since this adds a SECURITY DEFINER function reachable by
-- `authenticated`. It passes because the function resolves the caller
-- through `current_profile()` rather than taking anyone's word for it.
do $gate$
declare n int;
begin
  select count(*) into n from public.security_doors();
  if n > 0 then
    raise exception '198: the wall has % hole(s)', n;
  end if;
end
$gate$;
