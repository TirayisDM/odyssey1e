-- 078. A CLASS GRANTS ITS SAVES.
--
-- `classes.saving_throws` has existed since 055. It is parsed into the
-- `Class` struct, it is printed on the creation form's class picker,
-- and NOTHING HAS EVER WRITTEN IT TO A CHARACTER. Every character in
-- this game has been rolling saving throws short by their proficiency
-- bonus with nothing on screen to explain it.
--
-- The same shape as `price_override`, which 049 added and nothing
-- applied until 070: a column that exists to serve a rule, and the rule
-- never asks. Worth noticing that both were found by LOOKING AT A
-- SCREEN rather than by a test - a column nobody reads has nothing to
-- fail.
--
-- The engine applies this going forward in `apply_class_saves`. This is
-- the backfill for everybody who already exists.
--
-- ---------------------------------------------------------------------
-- THE STARTING CLASS ONLY
-- ---------------------------------------------------------------------
--
-- 5e's most asymmetric multiclassing rule, and the one
-- `multiclass::saves_granted` exists to state: the FIRST class grants
-- all of its saving throws and every later class grants NONE. A Fighter
-- 4 who takes a level of Bard keeps Strength and Constitution and gets
-- neither Dexterity nor Charisma.
--
-- The reason is that save proficiency is the strongest thing a class
-- hands out, and letting it accumulate would make a one-level dip the
-- cheapest defence in the game.
--
-- `added_at` orders them - the same column the starting-class hit die
-- reads in `multiclass::hp`, because "which did you start as" is one
-- fact serving two rules.
--
-- NOT THE LEADING CLASS. A Bard 1 / Fighter 9 reads as a fighter on
-- their sheet - `multiclass::primary` says so - and still saves as a
-- bard. The two questions have the same shape and different answers,
-- which is why a test pins exactly that pairing.
--
-- ---------------------------------------------------------------------
-- ADDITIVE, AND WHY
-- ---------------------------------------------------------------------
--
-- Sets `save_prof` TRUE and never false. A species, a feat or the DM's
-- own tick may have granted a save this does not know about, and a
-- re-derivation that cleared the rest would quietly undo all three. The
-- cost is that swapping a starting class leaves the old one's saves
-- behind - a tick to undo, rather than a wrong answer nobody can see.
--
-- `ca.ability` is the `ability_code` ENUM and `saving_throws` is
-- `text[]`, so the comparison needs an explicit cast. Postgres has no
-- operator for enum = text and refuses rather than guessing, which is
-- the same service `items_kind_check` did in 074.
--
-- The game's own class row wins over the global one of the same key -
-- `distinct on` with `game_id` sorted nulls last - which is the
-- precedence `class::load_map` applies in Rust.

with starting as (
  select distinct on (cc.character_id)
         cc.character_id,
         cc.class_key,
         c.game_id as game_id
    from character_classes cc
    join characters c on c.id = cc.character_id
   order by cc.character_id, cc.added_at asc, cc.class_key asc
),
resolved as (
  select distinct on (s.character_id)
         s.character_id,
         cl.saving_throws
    from starting s
    join classes cl
      on cl.key = s.class_key
     and (cl.game_id is null or cl.game_id = s.game_id)
   order by s.character_id, cl.game_id asc nulls last
)
update character_abilities ca
   set save_prof = true
  from resolved r
 where ca.character_id = r.character_id
   and ca.ability::text = any(r.saving_throws);
