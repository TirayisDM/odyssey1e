-- 155. AN ACTION REMEMBERS WHO TOOK IT.
--
-- Every cast in the encounter log reads "Someone":
--
--   Someone   SPIRITUAL WEAPON - +6 TO HIT, 1D8
--   Someone   SACRED FLAME - DEX SAVE DC 14, 1D8
--
-- 021 fixed exactly this sentence for ROLLS and the fix did not reach
-- here, because the log takes its name from the to-hit roll row:
--
--   who.textContent = (hit && hit.character_name) || "Someone";
--
-- A cast writes an action and NO rolls - that is the whole of 110's
-- two-step design, where the spell is cast and the d20 is thrown
-- separately - so there is no roll to carry the name, and the action
-- row has known the `character_id` all along without anybody asking it.
--
-- ---------------------------------------------------------------------
-- A TRIGGER, AND NOT A COLUMN IN `write_action`
-- ---------------------------------------------------------------------
--
-- 021's header is the best thing written in this folder and it says:
-- "an explicit column list in a writer function is a SECOND schema that
-- has to be kept in step with the first, and nothing checks it". It had
-- to add `character_name` to `write_action` as well as to `rolls`,
-- because that function names its columns and silently drops anything
-- not listed.
--
-- A BEFORE INSERT TRIGGER NEEDS NOTHING FROM THE WRITER. It fires on
-- the row whatever columns the insert happened to name, so this one
-- fact cannot be dropped in transit by `write_action`, by
-- `cast_prayer`'s plain REST insert, or by whatever writes an action
-- next. `write_action` is deliberately NOT touched by this migration.
--
-- THE SAME SHAPE 001 USED, down to the 'Someone' sentinel: a name that
-- arrives null, blank or 'Someone' is replaced, and anything else is
-- left alone so a caller that knows better than the characters table -
-- an actor's own label - still wins.

alter table public.actions
  add column if not exists character_name text not null default 'Someone';

create or replace function public.snapshot_action_name()
returns trigger
language plpgsql
-- SECURITY DEFINER for 001's reason: the row being named may belong to
-- somebody the inserting user cannot read in full.
security definer set search_path = ''
as $$
declare
  v text;
begin
  if new.character_name is null
     or btrim(new.character_name) in ('', 'Someone') then
    if new.character_id is not null then
      select c.name into v from public.characters c where c.id = new.character_id;
      new.character_name := coalesce(nullif(btrim(v), ''), 'Someone');
    end if;
  end if;
  return new;
end;
$$;

drop trigger if exists actions_snapshot_name on public.actions;
create trigger actions_snapshot_name
  before insert on public.actions
  for each row execute function public.snapshot_action_name();

-- ---------------------------------------------------------------------
-- AND THE ONES ALREADY WRITTEN
-- ---------------------------------------------------------------------
--
-- 001 SAYS A ROLL IS A RECORD AND NAMES ARE SNAPSHOTTED, NEVER
-- REWRITTEN - so this is the one moment it is allowed: these rows never
-- held a name to preserve, and taking the character's CURRENT name is
-- the only answer available. A creature renamed since will log under
-- the new name for its old actions, which is wrong in a small way and
-- less wrong than "Someone".
update public.actions a
   set character_name = coalesce(nullif(btrim(c.name), ''), 'Someone')
  from public.characters c
 where c.id = a.character_id
   and btrim(coalesce(a.character_name, '')) in ('', 'Someone');
