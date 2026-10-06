-- 123. A CREATURE TEMPLATE IS A CHARACTER.
--
-- 022 settled the instance: every actor in an encounter is a
-- `characters` row with `is_npc` true, so a goblin has ability rows,
-- hit points, death saves, equipment and resistances through the same
-- code a player character uses. Two goblins off one statblock die
-- separately because they are two characters rather than two views of
-- one.
--
-- WHAT NEVER CONVERGED IS THE TEMPLATE. `npcs` is a second, thinner
-- schema - 23 flat columns against `characters` 44 and eight satellite
-- tables - and it shares only ten column names with the thing it makes.
-- Abilities are columns there and rows here. There is nowhere on a
-- statblock to put a skill proficiency, a prepared spell, a class
-- feature with uses, a subclass choice or a multiclass level. A
-- creature gains the capacity for all of them the instant it is
-- instantiated and arrives with none.
--
-- SO A TEMPLATE IS A CHARACTER NOW. `is_template` marks it, and placing
-- one into an encounter is a character-to-character copy. Everything
-- built for a player character works on a creature the day this lands:
-- the ability rows, the save buttons, the resistance block, the
-- equipment ladder, prayers and slots.
--
-- `npcs` BECOMES A PUBLISHED REFERENCE rather than a live schema - the
-- Monster Manual, which you copy out of. It is not dropped and not
-- changed; `instantiate_npc` still reads it, and gains the ability to
-- land a template rather than a combatant.
--
-- ---------------------------------------------------------------------
-- A TEMPLATE IS NOT A CREATURE IN THE WORLD
-- ---------------------------------------------------------------------
-- It has no location, never takes a turn, never appears in a target
-- list, never rests and is nobody's audience. Six reads sweep a whole
-- game and every one of them now excludes templates; enrolment refuses
-- one outright. The Rust carries those guards - this migration carries
-- the column, the copy, and the one guard that has to be in the
-- database because `enrol_actor` is not the only way to write an
-- `encounter_actors` row.

alter table public.characters
  add column if not exists is_template boolean not null default false;

comment on column public.characters.is_template is
  'TRUE for a creature template - a thing you copy from rather than a thing in the world. It has no location, never takes a turn, never appears in a target list, never rests and is nobody''s audience. Placing one into an encounter copies it; see instantiate_character.';

-- The common question is "the templates in this game", and it is asked
-- every time the Creatures tab paints.
create index if not exists characters_templates
  on public.characters (game_id) where is_template;

-- ---------------------------------------------------------------------
-- THE GUARD THAT BELONGS IN THE DATABASE
-- ---------------------------------------------------------------------
-- The Rust keeps templates out of every list, which is where a guard
-- belongs for anything a person reads. But a template in an encounter
-- is not a display mistake, it is a corrupt fight - it would roll
-- initiative, take damage and die - and `encounter_actors` can be
-- written by anything holding the publishable key. So this one is a
-- trigger, for the same reason `character_slots` has a CHECK beside the
-- rule in Rust.
create or replace function public.no_templates_in_encounters()
returns trigger
language plpgsql
security definer
set search_path to 'public'
as $function$
begin
  if exists (
    select 1 from public.characters c
     where c.id = new.character_id and c.is_template
  ) then
    raise exception
      'that is a creature template - copy it into the encounter rather than enrolling it';
  end if;
  return new;
end;
$function$;

drop trigger if exists encounter_actors_no_templates on public.encounter_actors;
create trigger encounter_actors_no_templates
  before insert or update on public.encounter_actors
  for each row execute function public.no_templates_in_encounters();

-- ---------------------------------------------------------------------
-- PLACING A TEMPLATE IS A COPY
-- ---------------------------------------------------------------------
--
-- WHAT COMES WITH IT: the row, its ability scores, its skill
-- proficiencies, its class levels, the choices it has made, the spells
-- it has prepared, and its kit - including what is inside its
-- containers, which is why the object copy is recursive.
--
-- WHAT DOES NOT: spent spell slots, spent feature uses, hit point
-- events, death saves. A creature arrives rested and whole, which is
-- what "place a goblin" means. A template that has been poked at in
-- the Creatures tab does not bleed onto the creatures made from it.
--
-- THE TEMPLATE IS NEVER MODIFIED. Everything here reads it and writes
-- somewhere new.
create or replace function public.instantiate_character(
  p_source  uuid,
  p_game_id uuid,
  p_label   text
) returns uuid
language plpgsql
security definer
set search_path to 'public'
as $function$
declare
  src public.characters%rowtype;
  cid uuid;
  eid uuid;
begin
  select * into src from public.characters where id = p_source;
  if src.id is null then
    raise exception 'no such creature to copy';
  end if;

  insert into public.characters (
    game_id, owner_uid, name, is_npc, is_template,
    level, prof_bonus, hp_max, ac_mode, ac_override, size,
    weapon_profs, armor_profs, tool_profs,
    class_key, species_key, npc_key, creature_type,
    narrative_pack, markup, disposition,
    height_ft, weight_lb, hair, skin, eyes, description, languages)
  values (
    p_game_id,
    (select g.dm_uid from public.games g where g.id = p_game_id),
    coalesce(nullif(btrim(p_label), ''), src.name),
    true,
    false,
    src.level, src.prof_bonus, src.hp_max, src.ac_mode, src.ac_override, src.size,
    src.weapon_profs, src.armor_profs, src.tool_profs,
    src.class_key, src.species_key, src.npc_key, src.creature_type,
    src.narrative_pack, src.markup, src.disposition,
    src.height_ft, src.weight_lb, src.hair, src.skin, src.eyes,
    src.description, src.languages)
  returning id, entity_id into cid, eid;

  -- ABILITIES ARE UPDATED, NOT INSERTED: a trigger seeds six rows on
  -- every new character, so inserting would collide on the primary key.
  -- Same shape instantiate_npc has used since 022.
  update public.character_abilities a
     set score = s.score, save_prof = s.save_prof
    from public.character_abilities s
   where s.character_id = p_source
     and a.character_id = cid
     and a.ability = s.ability;

  insert into public.character_skills (character_id, skill_key, prof)
  select cid, s.skill_key, s.prof
    from public.character_skills s where s.character_id = p_source
  on conflict (character_id, skill_key) do update set prof = excluded.prof;

  insert into public.character_classes (character_id, class_key, level, hit_dice_spent)
  select cid, c.class_key, c.level, 0
    from public.character_classes c where c.character_id = p_source
  on conflict (character_id, class_key) do update set level = excluded.level;

  insert into public.character_choices (character_id, class_key, feature_key, pick, choice)
  select cid, c.class_key, c.feature_key, c.pick, c.choice
    from public.character_choices c where c.character_id = p_source;

  insert into public.character_prayers (character_id, spell_key, prepared)
  select cid, p.spell_key, p.prepared
    from public.character_prayers p where p.character_id = p_source
  on conflict (character_id, spell_key) do nothing;

  -- THE KIT, AND WHAT IS INSIDE IT. A flat copy would put a backpack on
  -- the new creature and leave its contents hanging off the template's
  -- entity - owned by nobody, visible in the old creature's pack. The
  -- recursion walks the whole tree and the join at the end re-points
  -- each copy at its own parent's new identity.
  with recursive tree as (
    select o.*, 0 as depth
      from public.objects o
     where o.holder_id = src.entity_id
    union all
    select o.*, t.depth + 1
      from public.objects o
      join tree t on o.holder_id = t.entity_id
     where t.depth < 10            -- 038's depth cap, and a cycle guard
  ),
  fresh as (
    select t.*,
           gen_random_uuid() as new_id,
           gen_random_uuid() as new_entity
      from tree t
  )
  insert into public.objects (
    id, entity_id, game_id, holder_id, item_key, quantity, name, slot,
    attuned, proficient_override, uses_spent, uses_max,
    size_override, holds_size_override, weight_override, price_override,
    damage_number_override, damage_denomination_override,
    damage_types_override, properties_override, base_ac_override, grants)
  select f.new_id, f.new_entity, p_game_id,
         coalesce(parent.new_entity, eid),
         f.item_key, f.quantity, f.name, f.slot,
         f.attuned, f.proficient_override, f.uses_spent, f.uses_max,
         f.size_override, f.holds_size_override, f.weight_override, f.price_override,
         f.damage_number_override, f.damage_denomination_override,
         f.damage_types_override, f.properties_override, f.base_ac_override, f.grants
    from fresh f
    left join fresh parent on parent.entity_id = f.holder_id;

  return cid;
end;
$function$;

-- ---------------------------------------------------------------------
-- AND A STATBLOCK CAN LAND AS A TEMPLATE
-- ---------------------------------------------------------------------
-- DROPPED AND RECREATED rather than overloaded. A fourth parameter with
-- a default would have left the three-argument function in place as an
-- exact match, so every existing call would have gone on reaching the
-- old body and the new flag would have done nothing - a silent no-op of
-- the usual kind. Both statements are in this migration's transaction,
-- so there is no window where the function is missing.
--
-- THE DEFAULT KEEPS EVERY EXISTING CALLER WORKING, including a client
-- built before today: three arguments still means "put a goblin in the
-- fight".
drop function if exists public.instantiate_npc(uuid, text, text);

create function public.instantiate_npc(
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

grant execute on function public.instantiate_character(uuid, uuid, text) to authenticated;
grant execute on function public.instantiate_npc(uuid, text, text, boolean) to authenticated;
