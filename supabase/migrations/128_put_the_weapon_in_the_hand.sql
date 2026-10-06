-- 128. PUT THE WEAPON IN THE HAND.
--
-- 127 filled the bestiary and the bestiary found a bug that has been in
-- `instantiate_npc` since 022. Three creatures arrive unable to swing
-- the weapon they are holding:
--
--   Hobgoblin   longsword -> NO SLOT
--   Wight       longsword -> NO SLOT
--   Bugbear     morningstar -> NO SLOT
--
-- and three more arrive holding the wrong thing:
--
--   Bandit      crossbow in hand, scimitar on the hip
--   Skeleton    shortsword on the hip, both hands empty
--   Sprite      (correct by luck - it is an archer)
--
-- IT NEVER SHOWED BECAUSE THERE WAS ONE GOBLIN. Handaxe and scimitar,
-- both equipped, both small: the axe took the hand and the scimitar
-- went to the hip, which is exactly right. Every assumption in the slot
-- logic happened to hold for the only creature that existed.
--
-- ---------------------------------------------------------------------
-- TWO FAULTS, AND THE FIRST CAUSES THE SECOND
-- ---------------------------------------------------------------------
--
-- THE RANK COUNTS THINGS THAT ARE NOT HELD. `wrank` is a row_number
-- over every kit row, so a carried bow takes rank 1 from the sword the
-- creature is actually using - and the sword, now rank 2, falls past
-- every branch. A skeleton ends up with its shortsword on its hip and
-- both hands empty because of a bow slung on its back.
--
-- A MEDIUM WEAPON HAS NOWHERE TO FALL. The final branches are "small
-- things go on the hip" and then nothing, so an equipped longsword that
-- is not rank 1 gets no slot at all. 084 made a null slot mean NOT
-- EQUIPPED, so that weapon is not merely misplaced - it is not in the
-- loadout, `attack::resolve` cannot see it, and the creature has no
-- attack.
--
-- RANK ONLY WHAT IS HELD, and give a second held weapon the off hand
-- when no shield wants it. A creature holding a shield and two melee
-- weapons still stows one, which is correct - two hands is two hands.

create or replace function public.instantiate_npc(
  p_game_id    uuid,
  p_npc_key    text,
  p_label      text,
  p_as_template boolean default false
) returns uuid
language plpgsql
security definer
set search_path to 'public'
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
    game_id, owner_uid, name, is_npc, is_template, level, prof_bonus,
    hp_max, ac_mode, ac_override, size,
    weapon_profs, armor_profs,
    class_key, npc_key, creature_type)
  values (
    p_game_id,
    (select g.dm_uid from public.games g where g.id = p_game_id),
    coalesce(nullif(btrim(p_label), ''), n.name),
    true,
    p_as_template,
    n.level,
    n.prof_bonus,
    n.hp_max,
    'flat',
    n.ac,
    n.size,
    n.weapon_profs,
    n.armor_profs,
    n.class_key,
    p_npc_key,
    n.creature_type)
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
           -- NOT HELD IS NOT EQUIPPED, and 084 says a null slot is
           -- exactly that. A javelin on the back is carried.
           when not p.equipped                         then null
           when p.armor_category = 'shl'               then 'left_hand'
           when p.kind = 'armor'                       then 'body'
           when p.kind = 'weapon' and p.wrank = 1      then 'right_hand'
           -- THE OFF HAND, when no shield has claimed it. A creature
           -- with a shield and two melee weapons stows one, which is
           -- what two hands means.
           when p.kind = 'weapon' and p.wrank = 2
                and not p.has_shield                   then 'left_hand'
           when p.size in ('tiny', 'sm')               then 'hip'
           else null
         end,
         p.proficient_override
    from (
      select k.*, it.kind, it.armor_category, it.size,
             -- RANKED AMONG WHAT IS HELD. Counting carried rows here is
             -- what let a slung bow take the hand off a drawn sword.
             case when k.equipped and it.kind = 'weapon'
                  then row_number() over (
                         partition by (k.equipped and it.kind = 'weapon')
                         order by k.item_key)
             end as wrank,
             bool_or(it.armor_category = 'shl' and k.equipped) over () as has_shield
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
$function$;

grant execute on function public.instantiate_npc(uuid, text, text, boolean) to authenticated;

-- ---------------------------------------------------------------------
-- AND ONE ROW OF 127 THAT SAID THE WRONG THING
-- ---------------------------------------------------------------------
-- The bandit was given a crossbow IN HAND alongside its scimitar. Every
-- other creature in 127 carries its ranged weapon and holds its melee
-- one; the bandit should too, and with the ranking fixed it would
-- otherwise draw the crossbow and leave the sword on its hip.
update public.npc_items
   set equipped = false
 where game_id is null and npc_key = 'bandit' and item_key = 'crossbow_light';
