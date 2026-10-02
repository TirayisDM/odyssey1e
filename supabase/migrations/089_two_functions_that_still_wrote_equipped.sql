-- 089. TWO FUNCTIONS THAT STILL WROTE `equipped`.
--
-- 084 dropped `objects.equipped` and migrated every Rust reader. It did
-- not migrate the two DATABASE functions that also write it, because
-- grepping the Rust found the Rust. Both broke immediately and one of
-- them broke in a way nothing would have caught without using the app:
--
--   put_in_container -> record "new" has no field "equipped" (400)
--
-- That is `unheld_is_unequipped`, a BEFORE trigger on objects. Moving
-- an object into a container changes its holder, the trigger fires, and
-- the whole write fails. Dropping a column is not finished when the
-- code that reads it compiles - plpgsql resolves its field names at RUN
-- TIME, so a trigger referencing a column that no longer exists is a
-- clean build and a broken button.
--
-- `instantiate_npc` had the same fault and would have failed the moment
-- a DM put a monster into a fight.

-- ---------------------------------------------------------------------
-- NOBODY IS HOLDING IT, SO IT IS NOT WORN
-- ---------------------------------------------------------------------
--
-- The same rule 030 wrote, said in 084's vocabulary: a slot is where
-- something is worn ON SOMEBODY, so an object on the floor or inside a
-- chest has none. Attunement goes with it, as before.
create or replace function unheld_is_unequipped()
returns trigger
language plpgsql
security definer
set search_path = public
as $$
begin
  if new.holder_id is null
     or not exists (select 1 from public.characters c where c.entity_id = new.holder_id)
  then
    new.slot    := null;
    new.attuned := false;
  end if;
  return new;
end;
$$;

-- ---------------------------------------------------------------------
-- A MONSTER'S KIT, PLACED
-- ---------------------------------------------------------------------
--
-- `npc_items.equipped` SURVIVES and is still the right shape there: a
-- statblock's kit is a PATTERN of what a monster carries, and a pattern
-- has no hands to put anything in. 084 only dropped the boolean on the
-- objects that are made FROM it.
--
-- So this is where the pattern becomes a placement, and it uses the
-- same rule 086's corrected backfill does: armour on the body, a shield
-- in the left hand, the first weapon in the right, and anything else
-- small at the belt. ONE weapon in hand, because a goblin carrying two
-- would otherwise be born in a state slots::check refuses.
create or replace function instantiate_npc(p_game_id uuid, p_npc_key text, p_label text)
returns uuid
language plpgsql
security definer
set search_path = public
as $$
declare
  n   public.npcs%rowtype;
  cid uuid;
  eid uuid;
begin
  select * into n
    from public.npcs
   where key = p_npc_key
     and (game_id is null or game_id = p_game_id)
   order by game_id nulls last
   limit 1;

  if n.key is null then
    raise exception 'no statblock with key %', p_npc_key;
  end if;

  insert into public.characters (
    game_id, owner_uid, name, is_npc, level, prof_bonus,
    hp_max, ac_mode, ac_override, size,
    weapon_profs, armor_profs,
    class_key)
  values (
    p_game_id,
    (select g.dm_uid from public.games g where g.id = p_game_id),
    coalesce(nullif(btrim(p_label), ''), n.name),
    true,
    n.level,
    n.prof_bonus,
    n.hp_max,
    'flat',
    n.ac,
    n.size,
    n.weapon_profs,
    n.armor_profs,
    n.class_key)
  returning id, entity_id into cid, eid;

  update public.character_abilities a
     set score = case a.ability::text
                   when 'str' then n.str when 'dex' then n.dex
                   when 'con' then n.con when 'int' then n.intl
                   when 'wis' then n.wis when 'cha' then n.cha
                 end
   where a.character_id = cid;

  insert into public.objects
    (game_id, holder_id, item_key, quantity, slot, proficient_override)
  select p_game_id, eid, p.item_key, p.quantity,
         case
           when not p.equipped                         then null
           when p.armor_category = 'shl'               then 'left_hand'
           when p.kind = 'armor'                       then 'body'
           when p.kind = 'weapon' and p.wrank = 1      then 'right_hand'
           when p.size in ('tiny', 'sm')               then 'hip'
           else null
         end,
         p.proficient_override
    from (
      select k.*, it.kind, it.armor_category, it.size,
             row_number() over (
               order by case when it.kind = 'weapon' then 0 else 1 end, k.item_key
             ) as wrank
        from (
          select distinct on (i.item_key) i.*
            from public.npc_items i
           where i.npc_key = p_npc_key
             and (i.game_id is null or i.game_id = p_game_id)
           order by i.item_key, i.game_id nulls last
        ) k
        -- LATERAL, not a plain join: a key can match a global row and a
        -- game's own, and a join would make two objects out of one
        -- kit entry. Same precedence as everywhere else - the game's
        -- row wins.
        left join lateral (
          select it.kind, it.armor_category, it.size
            from public.items it
           where it.key = k.item_key
             and (it.game_id is null or it.game_id = p_game_id)
           order by it.game_id nulls last
           limit 1
        ) it on true
    ) p;

  return cid;
end;
$$;

revoke all on function unheld_is_unequipped() from public;
revoke all on function instantiate_npc(uuid, text, text) from public;
grant execute on function instantiate_npc(uuid, text, text) to authenticated;
