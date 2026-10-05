-- 121. A MONSTER CAN SAY WHAT IT RESISTS.
--
-- 116 gave the engine resistance and wired it to four sources: a
-- species, a class feature, a spell and an item. It did not wire it to
-- the one that matters most at a table. The commonest resistance in 5e
-- is not on a player at all - it is "resistant to bludgeoning, piercing
-- and slashing from nonmagical attacks" on half the Monster Manual, and
-- `npcs` had no column to say it with.
--
-- ---------------------------------------------------------------------
-- A STATBLOCK STATES IT, AN INSTANCE LOOKS IT UP
-- ---------------------------------------------------------------------
--
-- MIRRORING `species_key`, which has done exactly this since 056: the
-- character row carries a KEY and the trait is read off the catalogue
-- when the sheet loads. A statblock corrected is every goblin
-- corrected, without a backfill and without touching a creature
-- mid-fight.
--
-- THE ALTERNATIVE WAS A `grants` COLUMN ON `characters`, copied at
-- instantiation. Rejected twice over: it is a snapshot, so fixing a
-- statblock typo leaves every goblin already on the board wrong; and a
-- general grants column on a character would be read by the resistance
-- path and by nothing else, which is a silent no-op waiting for
-- somebody to write an `ac` target into it and wonder why their armour
-- class did not move.
--
-- `characters.npc_key` IS INFORMATION THAT WAS BEING THROWN AWAY.
-- `instantiate_npc` read a statblock, copied six numbers out of it and
-- forgot where they came from, so nothing downstream could ever ask the
-- statblock another question. This is the first of those questions and
-- it will not be the last.

-- ---------------------------------------------------------------------
-- THE COLUMNS FIRST. 089's LESSON.
-- ---------------------------------------------------------------------
-- plpgsql resolves field names at RUN time, so a function naming a
-- column that does not exist yet compiles clean and fails on the
-- button. The columns are added before the function that writes one, in
-- that order, in one migration.

alter table public.npcs
  add column if not exists grants jsonb not null default '[]'::jsonb;

comment on column public.npcs.grants is
  'The grant vocabulary from 100, for what this creature IS rather than what it carries. Resistance is the first use - a target of resist.fire, immune.poison or vulnerable.cold, with the thirteen damage types and the spelling owned by resist.rs. Kit goes in npc_items; this is the statblock''s own traits.';

alter table public.characters
  add column if not exists npc_key text;

comment on column public.characters.npc_key is
  'Which statblock this creature was instantiated from, or NULL for a player character. A KEY AND NOT A COPY, the same arrangement species_key has had since 056: traits are read off `npcs` when the sheet loads, so correcting a statblock corrects every creature made from it. By value with no foreign key - npcs carries the same nullable game_id tenancy as every other catalogue, and a partial unique index cannot back one.';

create index if not exists characters_by_npc_key
  on public.characters (npc_key) where npc_key is not null;

-- ---------------------------------------------------------------------
-- WHAT IS ALREADY ON THE BOARD
-- ---------------------------------------------------------------------
-- Best effort, and it costs nothing where it misses. 018 names an
-- instance "<statblock name> 0001", so the trailing ordinal comes off
-- and the rest is matched against a statblock within the same game's
-- reach. A creature hand-named before 018 will not match and stays
-- NULL, which reads as "no statblock traits" - exactly what it had
-- yesterday.
update public.characters c
   set npc_key = n.key
  from public.npcs n
 where c.is_npc
   and c.npc_key is null
   and (n.game_id is null or n.game_id = c.game_id)
   and lower(btrim(regexp_replace(c.name, '[[:space:]]+[0-9]+$', '')))
     = lower(btrim(n.name));

-- ---------------------------------------------------------------------
-- AND THE FUNCTION REMEMBERS WHERE IT CAME FROM
-- ---------------------------------------------------------------------
-- Replaced whole, because that is the only way to change a plpgsql
-- function. The ONLY difference from the version 095 left is `npc_key`
-- in the insert's column list and `p_npc_key` in its values; everything
-- else is 095 verbatim, including the LATERAL join and the note saying
-- why it is lateral.
create or replace function public.instantiate_npc(p_game_id uuid, p_npc_key text, p_label text)
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
    game_id, owner_uid, name, is_npc, level, prof_bonus,
    hp_max, ac_mode, ac_override, size,
    weapon_profs, armor_profs,
    class_key, npc_key)
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
    n.class_key,
    p_npc_key)
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
$function$;

-- NOTHING IS SEEDED, AND THAT IS NOT AN OVERSIGHT. The catalogue holds
-- three statblocks and all three are goblins, which resist nothing in
-- 5e. Inventing a monster to demonstrate a column would be putting game
-- content into a schema migration. Giving one a resistance is one
-- statement:
--
--   update npcs
--      set grants = '[{"target":"resist.poison","source":"Goblin"}]'
--    where key = 'goblin';
