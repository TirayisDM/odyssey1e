-- 177. A PRAYER AND A SPELL ARE BOTH A SPELL.
--
-- Dave: "wizards get spells, clerics get prayers." The rules have said
-- so since 150 - `casting::Source` is `WholeList` or `Book` and
-- `may_reach` is the one function that branches - and every NAME still
-- said cleric. 176 renamed the Rust: `prayers.rs` became `casting.rs`,
-- `cleric_level` became `caster_level`, `wis_mod` became `ability_mod`
-- (a wizard's INTELLIGENCE had been going into that one), and the four
-- commands became `list_casting`, `prepare_spell`, `forget_spell` and
-- `cast_spell`.
--
-- This is the last of it, and the only part with teeth.
--
-- ---------------------------------------------------------------------
-- WHY THIS IS ITS OWN MIGRATION
-- ---------------------------------------------------------------------
--
-- 152 IS THE RECORD OF GETTING THIS WRONG. It dropped
-- `character_prayers.prepared` after checking only the Rust readers,
-- and `instantiate_character` - plpgsql, from 123 - had been reading
-- it. Every enrol broke. PLPGSQL RESOLVES A STATEMENT'S COLUMNS ON
-- FIRST EXECUTION, NOT AT CREATION, so a function left pointing at a
-- renamed table compiles, deploys, and fails the first time somebody
-- enrols a goblin.
--
-- A table rename is strictly larger than a dropped column, so the
-- inventory was taken from the catalogue rather than from memory:
--
--   4 RLS policies          renamed below
--   3 constraints           renamed below
--   0 triggers              nothing to do
--   0 views                 nothing to do
--   2 plpgsql functions     RECREATED below, in this migration
--   64 rows                 carried by the rename, untouched
--
-- The functions are `instantiate_character` and `instantiate_npc`,
-- found with `prosrc ilike '%character_prayers%'` over `pg_proc` -
-- which is the check 152 did not do.
--
-- 095'S RULE HOLDS: BOTH SIGNATURES ARE BYTE-FOR-BYTE. PostgREST
-- dispatches an RPC on PARAMETER NAMES, so a `create or replace` that
-- renames or reorders an argument does not replace anything - it
-- creates a second overload, and every call then fails as ambiguous.
-- `instantiate_npc(p_game_id, p_npc_key, p_label, p_as_template)` keeps
-- that order, those names, and the `default false` on the last one.
-- The bodies below are `pg_get_functiondef`'s own output with the table
-- name changed and nothing else.
--
-- ---------------------------------------------------------------------
-- WHAT IS NOT RENAMED
-- ---------------------------------------------------------------------
--
-- `state` KEEPS ITS THREE WORDS - 'cantrip', 'prepared', 'book' - and
-- 'book' is still the word for written down and not up today. 172 moved
-- the WRITING to `scribed_spells` on the object, and `load_chosen`
-- folds what a carried book contains into this table's states, so
-- 'book' here now means "reachable from a book" rather than "a row
-- somebody wrote". That is 172's design and not this migration's to
-- revisit.
--
-- `npc_spells` was already named right in 150.

alter table public.character_prayers rename to character_spells;

-- A renamed table does NOT rename its own constraints or indexes, and a
-- constraint called `character_prayers_pkey` on a table called
-- `character_spells` is the same lie this migration exists to remove.
alter table public.character_spells
  rename constraint character_prayers_pkey to character_spells_pkey;
alter table public.character_spells
  rename constraint character_prayers_character_id_fkey
    to character_spells_character_id_fkey;
alter table public.character_spells
  rename constraint character_prayers_state_check
    to character_spells_state_check;

-- The policy EXPRESSIONS need no change: they refer to the table by
-- oid, so they already read `character_spells.character_id`. Only the
-- names are text.
alter policy "character_prayers: read with character"
  on public.character_spells rename to "character_spells: read with character";
alter policy "character_prayers: owner or dm writes"
  on public.character_spells rename to "character_spells: owner or dm writes";
alter policy "character_prayers: owner or dm updates"
  on public.character_spells rename to "character_spells: owner or dm updates";
alter policy "character_prayers: owner or dm deletes"
  on public.character_spells rename to "character_spells: owner or dm deletes";

-- ---------------------------------------------------------------------
-- THE TWO FUNCTIONS, BEFORE ANYBODY CALLS THEM
-- ---------------------------------------------------------------------

create or replace function public.instantiate_character(p_source uuid, p_game_id uuid, p_label text)
 returns uuid
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

  -- 152. `state`, AND CARRIED ACROSS RATHER THAN DECIDED. 'cantrip',
  -- 'prepared' or 'book' - whatever the source held, the copy holds.
  insert into public.character_spells (character_id, spell_key, state)
  select cid, p.spell_key, p.state
    from public.character_spells p where p.character_id = p_source
  on conflict (character_id, spell_key) do nothing;

  perform public.copy_kit(src.entity_id, eid, p_game_id, 0);

  return cid;
end;
$function$;

create or replace function public.instantiate_npc(p_game_id uuid, p_npc_key text, p_label text, p_as_template boolean default false)
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

  -- 150. THE CLASS ROW, which is what makes a creature cast at all.
  -- Only when the statblock states a level: class_key alone has been a
  -- label since 064, and giving every labelled creature class features
  -- it was never designed with would be a balance change arriving as a
  -- side effect, which is 129's mistake.
  if n.class_key is not null and n.class_level > 0 then
    insert into public.character_classes (character_id, class_key, level)
    values (cid, n.class_key, n.class_level);
  end if;

  -- 150. AND WHAT IT KNOWS. Same shape as the kit above: a pattern on
  -- the statblock, copied onto the individual, so editing the type
  -- never reaches a creature already made from it.
  insert into public.character_spells (character_id, spell_key, state)
  select cid, s.spell_key, s.state
    from (
      select distinct on (sp.spell_key) sp.*
        from public.npc_spells sp
       where sp.npc_key = p_npc_key
         and (sp.game_id is null or sp.game_id = p_game_id)
       order by sp.spell_key, sp.game_id nulls last
    ) s;

  return cid;
end;
$function$;
