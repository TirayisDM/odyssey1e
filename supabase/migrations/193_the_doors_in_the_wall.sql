-- 193. THE DOORS IN THE WALL.
--
-- RLS is the wall. Every SECURITY DEFINER function is a door through
-- it, because that is what SECURITY DEFINER means: run as the owner and
-- do not consult the policies. There are 30 such functions and, until
-- this migration, exactly one of them - `join_game` - looked at who was
-- knocking.
--
-- ---------------------------------------------------------------------
-- WHAT WAS ACTUALLY OPEN
-- ---------------------------------------------------------------------
--
-- Five functions carried the DEFAULT PUBLIC grant:
--
--   copy_kit  instantiate_character  instantiate_npc
--   no_templates_in_encounters  snapshot_action_name
--
-- The last two are trigger functions and harmless to call directly -
-- they reference NEW and fail outside a trigger. The first three are
-- not. They bypass RLS by design, check nothing about the caller, and
-- `anon` could execute them:
--
--   anyone holding the publishable key - which ships inside the app -
--   could create characters and NPCs in any game whose UUID they knew,
--   and copy kit between any two holders, WITHOUT SIGNING IN.
--
-- The only thing standing in the way was that UUIDs are hard to guess.
-- Secrecy of an identifier is not an authorization model.
--
-- ---------------------------------------------------------------------
-- WHY IT HAPPENED, AND WHY IT WOULD HAVE HAPPENED AGAIN
-- ---------------------------------------------------------------------
--
-- 002 is called "tighten function grants" and did exactly this job. Every
-- function written since has quietly undone it, because in Postgres:
--
--   grant execute on function f to authenticated
--
-- does NOT revoke the default grant to PUBLIC. It adds a grant beside a
-- door that is already open. 152 wrote that line believing it was the
-- permission; it was an addition to one.
--
-- SO THE LAST STATEMENT IN THIS FILE IS THE ONE THAT MATTERS.
-- `alter default privileges` stops new functions being born public, which
-- is the only version of this fix that survives the next migration.
--
-- ---------------------------------------------------------------------
-- AND THE WALL ITSELF: PLACING A CREATURE IS A DM ACT
-- ---------------------------------------------------------------------
--
-- Dave's call. Both `instantiate_*` are reached from DM-only surfaces -
-- the Creatures tab, which hides its own button, and the Run panel - so
-- this refuses nothing that was being offered. What it closes is the
-- door BESIDE the screen: an authenticated member of one game could
-- create characters in another game entirely, because a SECURITY
-- DEFINER function never consulted a policy and these never asked.
--
-- THE BODIES ARE OTHERWISE UNTOUCHED. Everything below the guard is
-- `pg_get_functiondef` as it stood, and the migration is verified after
-- applying by diffing old against new: the guard is the only change.

create or replace function public.instantiate_character(
  p_source  uuid,
  p_game_id uuid,
  p_label   text
) returns uuid
language plpgsql
security definer
set search_path to 'public'
as $fn$
declare
  src public.characters%rowtype;
  cid uuid;
  eid uuid;
begin
  -- 193. THE CALLER IS THE DM OF THIS GAME, OR NOBODY. `is_game_dm`
  -- reads `auth.uid()`, which is the request's JWT and not this
  -- function's owner, so it sees the real caller even from inside a
  -- SECURITY DEFINER body. An unauthenticated call has no uid and is
  -- refused here rather than by luck.
  if not public.is_game_dm(p_game_id) then
    raise exception 'only the DM of this game may place a creature in it';
  end if;

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
$fn$;

create or replace function public.instantiate_npc(
  p_game_id    uuid,
  p_npc_key    text,
  p_label      text,
  p_as_template boolean default false
) returns uuid
language plpgsql
security definer
set search_path to 'public'
as $fn$
declare
  n   public.npcs%rowtype;
  cid uuid;
  eid uuid;
begin
  -- 193. SAME DOOR, SAME LOCK. See instantiate_character.
  if not public.is_game_dm(p_game_id) then
    raise exception 'only the DM of this game may place a creature in it';
  end if;

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
$fn$;

-- ---------------------------------------------------------------------
-- SHUT THE DOORS
-- ---------------------------------------------------------------------

-- Nothing in this schema is for the unauthenticated. The app's own
-- reads go through RLS with a signed-in token; `anon` exists only to
-- reach the auth endpoints.
revoke execute on all functions in schema public from public;
revoke execute on all functions in schema public from anon;

-- `copy_kit` IS INTERNAL. Nothing in the Rust calls it - only
-- `instantiate_character` does, from inside a SECURITY DEFINER body
-- that runs as the owner and therefore needs no grant of its own.
revoke execute on function
  public.copy_kit(uuid, uuid, uuid, integer) from authenticated;

-- ---------------------------------------------------------------------
-- AND KEEP THEM SHUT
-- ---------------------------------------------------------------------
--
-- THE ONLY PART OF THIS THAT SURVIVES THE NEXT MIGRATION. Everything
-- above fixes five functions; this fixes the habit. A function created
-- after this line is born with no PUBLIC grant, so forgetting to revoke
-- is no longer a way to open a door.
alter default privileges in schema public
  revoke execute on functions from public;
alter default privileges for role postgres in schema public
  revoke execute on functions from public;
