-- =====================================================================
-- 065_an_instance_inherits_its_calling.sql
-- odyssey1e — instantiate_npc carries the class across
-- =====================================================================
--
-- 064 put a class on the statblocks and swept the twelve instances that
-- already existed. The THIRTEENTH would have come out with none.
--
-- `instantiate_npc` copies ten columns from the statblock onto the new
-- character - name, level, proficiency bonus, hit points, armour class,
-- size and both proficiency lists - and it could not copy `class_key`
-- because that column did not exist when the function was written.
--
-- This is the same half-a-feature Dave has named twice: `objects.attuned`
-- since 008 and `characters.markup` since 040, both columns a rule
-- wanted and nothing could write. A sweep that fixes today and leaves
-- tomorrow broken is the same shape.
--
-- NOTHING ELSE ABOUT THE FUNCTION CHANGES. It is reproduced whole
-- because `create or replace` takes the whole body, not because any
-- other line was reconsidered.
-- =====================================================================

create or replace function public.instantiate_npc(p_npc_key text, p_game_id uuid, p_label text)
returns uuid
language plpgsql
set search_path to ''
as $function$
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
    -- 065. The eleventh column. A class gives the instance their
    -- attack count and a name for what they are; their hit points
    -- still come from n.hp_max above, which is 029's rule and not
    -- 061's - see the guard in rederive_hp_max.
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
    (game_id, holder_id, item_key, quantity, equipped, proficient_override)
  select p_game_id, eid, x.item_key, x.quantity, x.equipped,
         x.proficient_override
    from (
      select distinct on (i.item_key) i.*
        from public.npc_items i
       where i.npc_key = p_npc_key
         and (i.game_id is null or i.game_id = p_game_id)
       order by i.item_key, i.game_id nulls last
    ) x;

  return cid;
end;
$function$;
