-- =====================================================================
-- 028_statblock_proficiency.sql
-- odyssey1e — a goblin is trained, not merely equipped
-- =====================================================================
--
-- THE BUG THIS FIXES, found by handing Crumbs a sickle.
--
-- `characters` has carried weapon_profs and armor_profs since 008, and
-- `npcs` never got them. So `instantiate_npc` made a character whose
-- proficiency arrays were empty, and covered for it by stamping
-- `coalesce(x.proficient_override, true)` on every kit row it copied.
--
-- That reads as "a monster is proficient with its gear", which is true.
-- What it actually says is "a monster is proficient with exactly the
-- rows that came out of npc_items, and with nothing else, ever". Hand a
-- goblin a sickle and the derivation runs against an empty array and
-- says no - so a creature holding a simple weapon it has every business
-- knowing swings at STR alone:
--
--   Runt   · Scimitar  1d20 +4   DEX +2, PB +2   (the stamped override)
--   Crumbs · Sickle    1d20 -1   STR -1, no PB   (derived, against {})
--
-- Same campaign, same species, same round. The difference is entirely
-- which table the weapon arrived through, which is not a rule anyone
-- would write down.
--
-- WHAT THIS DOES. `npcs` gets the two arrays, `instantiate_npc` copies
-- them onto the character it makes, and the coalesce goes. Proficiency
-- for a monster is then decided by exactly the rule that already
-- decides it for Rodnar - equipment.rs::is_proficient, unchanged - and
-- the tri-state override goes back to meaning what 008 said it means:
-- an explicit answer from the source, not a default in disguise.
--
-- THE GOBLIN CARRIES A SCIMITAR, WHICH IS MARTIAL. That is the case
-- that decides the shape here, and there were two ways to take it:
--
--   weapon_profs {sim,mar}   - goblins are trained in martial weapons
--   weapon_profs {sim} + an override on the scimitar kit row
--
-- The second. A goblin is not a martial-weapon user in general; it
-- knows the blade it carries. Promoting the species would hand it
-- proficiency with a greatsword, a halberd and a longbow it has never
-- held, which is the same over-reach in the other direction. The
-- override is the right tool and this is the case 008 built it for.
--
-- ARMOUR follows the Monster Manual: leather and a shield, AC 15. The
-- statblock's ac is flat and nothing derives from these yet, but an
-- armour proficiency is a fact about the creature rather than about the
-- armour it happens to be wearing - and the moment a DM hands a goblin
-- a breastplate, this is the column that answers.
-- =====================================================================

alter table public.npcs add column weapon_profs text[] not null default '{}';
alter table public.npcs add column armor_profs  text[] not null default '{}';

comment on column public.npcs.weapon_profs is
  'What this creature is TRAINED with: sim, mar, or a bare baseItem granting one weapon. Same vocabulary and same meaning as characters.weapon_profs - instantiate_npc copies it across, and from there one rule decides proficiency for everyone. Before 028 this did not exist and instantiate_npc faked it by stamping every kit row proficient, which made a monster proficient with its starting gear and nothing else forever.';
comment on column public.npcs.armor_profs is
  'lgt med hvy shl, matching characters.armor_profs. Nothing derives from it while a statblock ac is flat, but it is a fact about the creature rather than about what it is wearing, and it is the column that answers when a DM hands a goblin a breastplate.';

-- ---------------------------------------------------------------------
-- The goblin, as the Monster Manual has it: simple weapons, leather and
-- a shield. The scimitar is martial and it knows that one.
-- ---------------------------------------------------------------------

update public.npcs
   set weapon_profs = array['sim']::text[],
       armor_profs  = array['lgt','shl']::text[]
 where key = 'goblin' and game_id is null;

update public.npc_items
   set proficient_override = true
 where npc_key = 'goblin' and item_key = 'scimitar';

-- The handaxe is deliberately left NULL. It is simpleM, `sim` covers
-- it, and an override there would be a second answer to a question the
-- derivation already gets right - exactly the redundancy that let this
-- bug hide.

-- ---------------------------------------------------------------------
-- instantiate_npc, without the coalesce.
-- ---------------------------------------------------------------------

create or replace function public.instantiate_npc(
  p_npc_key  text,
  p_game_id  uuid,
  p_label    text
)
returns uuid
language plpgsql
security invoker
set search_path = ''
as $$
declare
  n   public.npcs%rowtype;
  cid uuid;
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
    weapon_profs, armor_profs)
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
    n.armor_profs)
  returning id into cid;

  update public.character_abilities a
     set score = case a.ability::text
                   when 'str' then n.str when 'dex' then n.dex
                   when 'con' then n.con when 'int' then n.intl
                   when 'wis' then n.wis when 'cha' then n.cha
                 end
   where a.character_id = cid;

  -- The kit's override travels VERBATIM now. It used to arrive as
  -- coalesce(..., true), which made every copied row an explicit
  -- answer and left the derivation with nothing to do - see the header.
  insert into public.objects
    (game_id, character_id, item_key, quantity, equipped, proficient_override)
  select p_game_id, cid, x.item_key, x.quantity, x.equipped,
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
$$;

revoke all on function public.instantiate_npc(text, uuid, text) from public, anon;
grant execute on function public.instantiate_npc(text, uuid, text) to authenticated;

-- ---------------------------------------------------------------------
-- The monsters already on the table.
--
-- `characters` carries no npc_key, so the way back to a statblock is
-- through the actor that enrolled it. That is the only link there is,
-- and it is enough: every NPC character in the database got there via
-- instantiate_npc, called from enrol_actor.
-- ---------------------------------------------------------------------

-- A joined table's ON clause cannot see the UPDATE target, and a key
-- can match both a global statblock and a campaign's override of it -
-- two source rows for one character, which is an ambiguous update. The
-- subquery settles both: it picks one statblock per character, game
-- scoped winning, the same `nulls last` precedence every other read of
-- this table uses.
update public.characters c
   set weapon_profs = s.weapon_profs,
       armor_profs  = s.armor_profs
  from (
    select distinct on (a.character_id)
           a.character_id, n.weapon_profs, n.armor_profs
      from public.encounter_actors a
      join public.characters ch on ch.id = a.character_id
      join public.npcs n
        on n.key = a.npc_key
       and (n.game_id is null or n.game_id = ch.game_id)
     order by a.character_id, n.game_id nulls last
  ) s
 where s.character_id = c.id
   and c.is_npc
   and c.weapon_profs = '{}'
   and c.armor_profs  = '{}';

-- And the overrides the old coalesce manufactured. Only the rows whose
-- kit entry actually says nothing are cleared, so a genuine answer -
-- the goblin's scimitar, set above - survives.
--
-- An object the DM added later is not in any kit and is not matched, so
-- Crumbs' sickle keeps its NULL and starts being derived correctly
-- against the arrays this migration just filled in.
--
-- EXISTS rather than a join, because an object whose holder appears in
-- several encounters would otherwise be matched once per appearance.
-- The answer is the same every time and the row would be written
-- repeatedly to reach it.
update public.objects o
   set proficient_override = null
 where o.proficient_override is true
   and exists (
     select 1
       from public.characters c
       join public.encounter_actors a on a.character_id = c.id
       join public.npc_items k
         on k.npc_key = a.npc_key
        and k.item_key = o.item_key
      where c.id = o.character_id
        and c.is_npc
        and k.proficient_override is null
   );
