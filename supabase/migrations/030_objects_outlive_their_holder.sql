-- =====================================================================
-- 030_objects_outlive_their_holder.sql
-- odyssey1e — deleting a goblin should not destroy its sword
-- =====================================================================
--
-- A LEFTOVER FROM 008, and a correct one at the time. `character_items`
-- was keyed on a NOT NULL character_id, so an object without a holder
-- could not be represented at all - and given that, ON DELETE CASCADE
-- was the only honest answer. There was nowhere for an orphan to go.
--
-- 026 removed the premise and did not revisit the conclusion.
-- character_id became nullable, NULL came to mean "nobody is carrying
-- it", and the cascade went on destroying things anyway. So clearing
-- out some stale test goblins would have taken a scimitar, a handaxe
-- and a sickle with them - not because anyone decided gear should be
-- destroyed, but because a rule written for a different schema was
-- still running.
--
-- WHAT THIS DOES. SET NULL. Deleting a creature drops what it was
-- carrying: the objects stay in the campaign, held by nobody, exactly
-- as if someone had dropped them. `game_id` is NOT NULL and has been
-- since 026, which is what makes that a real place to be rather than an
-- orphan row - the campaign still owns it.
--
-- WHAT STILL CASCADES, and should. hp_events, character_abilities,
-- character_skills, character_dice and encounter_actors all go with the
-- character, because every one of them is a fact ABOUT that creature
-- and means nothing without it. An object is not: it is a thing in the
-- world that happened to be in someone's hands.
--
-- rolls and actions were already SET NULL - see 009 and 012. A roll is
-- a record and snapshots the name it was rolled under, so the log stays
-- readable when the character is gone. This puts objects in the same
-- category for the same reason: some things outlive the row that
-- pointed at them.
--
-- ---------------------------------------------------------------------
-- AND THE TRIGGER, which is the half that is easy to miss.
--
-- An object that becomes unheld must stop being equipped. `drop_object`
-- already said so in Rust, but a CASCADE happens inside Postgres where
-- no Rust runs - so without this, deleting a character would leave a
-- breastplate lying on the floor still flagged equipped, and
-- `take_object` would put it straight onto whoever picked it up.
-- Equipped, unchecked, bypassing the one-armor rule that
-- `set_item_equipped` exists to enforce.
--
-- That is why the rule moves HERE rather than being duplicated. The
-- database is the only place that can see every way an object comes
-- unheld; a second copy in Rust would be a second answer to maintain,
-- and this one would still be the one that ran.
-- =====================================================================

alter table public.objects
  drop constraint character_items_character_id_fkey;

alter table public.objects
  add constraint objects_character_id_fkey
  foreign key (character_id) references public.characters(id)
  on delete set null;

comment on column public.objects.character_id is
  'Who is holding it. NULL means nobody - the object exists in the campaign and is not carried. SET NULL on delete since 030: removing a creature drops its gear rather than destroying it, which the 008 cascade did because back then an unheld object could not be represented. There is still nowhere to PUT a dropped thing, because Locations is a stub.';

-- ---------------------------------------------------------------------
-- Nothing unheld is equipped.
-- ---------------------------------------------------------------------

create or replace function public.unheld_is_unequipped()
returns trigger
language plpgsql
security invoker
set search_path = ''
as $$
begin
  if new.character_id is null then
    new.equipped := false;
    new.attuned  := false;
  end if;
  return new;
end;
$$;

comment on function public.unheld_is_unequipped() is
  'Nothing nobody is holding is equipped or attuned. Stated here rather than in Rust because a CASCADE or SET NULL runs inside Postgres where no command does - and an unheld object left flagged equipped would be worn the instant somebody picked it up, bypassing the one-armor check.';

create trigger objects_unheld_is_unequipped
  before insert or update on public.objects
  for each row execute function public.unheld_is_unequipped();

-- Anything already in that state. There should be none - until 030 an
-- object could only become unheld through drop_object, which cleared
-- both flags itself - but a backfill that finds nothing costs nothing
-- and a wrong assumption here is silent.
update public.objects
   set equipped = false, attuned = false
 where character_id is null
   and (equipped or attuned);
