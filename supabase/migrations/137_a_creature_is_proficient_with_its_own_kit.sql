-- 137. A CREATURE IS PROFICIENT WITH ITS OWN BODY AND ITS OWN KIT.
--
-- Dave: all creature weapons and natural attacks should be proficient by
-- default, and anything added later in customization is set at that
-- time. The data was almost there already - 223 of 224 global kit rows
-- carry `proficient_override = true`, including all 125 natural weapons
-- - so this is mostly about making the DEFAULT true rather than
-- relying on every future seed to remember.
--
-- ---------------------------------------------------------------------
-- NULL MEANS TWO DIFFERENT THINGS TODAY, WHICH IS THE ACTUAL FAULT
-- ---------------------------------------------------------------------
--
-- `proficient_override` is a tri-state and the nullability is the point:
-- true or false came from somebody, NULL means nobody has said. Two
-- things read a NULL on a creature's kit and they do not agree:
--
--   equipment::load_npc_kit   NULL -> `unwrap_or(true)`, proficient
--   equipment::is_proficient  NULL -> derive from `weapon_profs`
--
-- So the DM's statblock preview and the instantiated creature can answer
-- differently about the same row, which is this codebase's named defect
-- with a to-hit attached rather than an error.
--
-- AND THE DERIVED ANSWER IS WRONG FOR A MONSTER. Several statblocks
-- carry `weapon_profs = {sim}` while holding something martial - the
-- Goblin holds a scimitar - so deriving says NOT proficient and takes
-- the proficiency bonus off its own weapon. That has not bitten anybody
-- yet only because 131 set the flag on every row it wrote. The one row
-- it did not write - 022's original goblin handaxe, the oldest kit row
-- in the schema - is NULL to this day, and survives because a handaxe
-- is simple and the derivation happens to come out true.
--
-- THE FIX IS NOT TO WIDEN `weapon_profs`. Giving the Goblin `mar` would
-- make the next martial weapon anybody hands it proficient too, which is
-- precisely what Dave asked NOT to happen. Proficiency with what a
-- statblock SHIPS is a different fact from proficiency with a class of
-- weapon, so it is stated per row and now the default states it.

alter table public.npc_items
  alter column proficient_override set default true;

comment on column public.npc_items.proficient_override is
  'Whether the creature is proficient with this kit row. DEFAULTS TO TRUE (137): a statblock ships what the creature is meant to use, and a monster is not untrained with its own teeth or its own sword - its weapon_profs classes are about what it could be GIVEN, not about what it was designed holding. Set false explicitly for a creature fumbling something it should not have. NULL still means nobody has said, which load_npc_kit reads as proficient and is_proficient derives, and is why instantiate_npc resolves it rather than passing it on.';

-- THE ONE ROW THAT WAS NEVER STATED. Scoped to the global rows where
-- nobody has said - 132's lesson about `game_id is null` reading like a
-- scope when it is a tenancy predicate: here it means "the shared
-- catalogue, where nothing was stated", and a game's own kit rows are
-- left to that game.

update public.npc_items
   set proficient_override = true
 where game_id is null
   and proficient_override is null;

-- ---------------------------------------------------------------------
-- AND THE OBJECTS A CREATURE ARRIVES HOLDING
-- ---------------------------------------------------------------------
--
-- `instantiate_npc` copied the kit row's NULL straight onto the object,
-- where `is_proficient` derives it. Resolving it here makes the stored
-- fact explicit, so the preview and the creature cannot disagree and
-- nothing downstream has to guess.
--
-- THE SIGNATURE IS BYTE-FOR-BYTE 124's. 095 had to drop a duplicate
-- `instantiate_npc` that a parameter reorder created - PostgREST
-- dispatches an RPC on parameter NAMES, so `create or replace` with the
-- arguments in a different order makes a SECOND function rather than
-- replacing the first, and every call then fails as ambiguous. The only
-- change in the body below is `coalesce(p.proficient_override, true)`.

create or replace function public.instantiate_npc(
  p_game_id uuid, p_npc_key text, p_label text, p_as_template boolean default false)
returns uuid
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
           when not p.equipped                         then null
           when p.armor_category = 'shl'               then 'left_hand'
           when p.kind = 'armor'                       then 'body'
           when p.kind = 'weapon' and p.wrank = 1      then 'right_hand'
           when p.kind = 'weapon' and p.wrank = 2
                and not p.has_shield                   then 'left_hand'
           when p.size in ('tiny', 'sm')               then 'hip'
           else null
         end,
         -- 137. PROFICIENT UNLESS THE STATBLOCK SAYS OTHERWISE. A NULL
         -- here used to reach the object and be derived from
         -- `weapon_profs`, which denies a Goblin its own scimitar.
         coalesce(p.proficient_override, true)
    from (
      select k.*, it.kind, it.armor_category, it.size,
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

-- AND THE CREATURES ALREADY STANDING. Two objects today - a Goblin
-- template and Goblin 0001, both holding 022's handaxe. Scoped to a
-- creature's OWN statblock kit, which is the part Dave said should be
-- proficient by default; anything added to a creature afterwards is
-- left exactly as the DM left it.

update public.objects o
   set proficient_override = true
 where o.proficient_override is null
   and exists (
     select 1
       from public.characters c
       join public.npc_items ni
         on ni.npc_key = c.npc_key
        and ni.item_key = o.item_key
        and (ni.game_id is null or ni.game_id = c.game_id)
      where c.entity_id = o.holder_id
        and c.npc_key is not null
   );
