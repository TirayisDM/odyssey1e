-- 124. COPY A KIT THE WAY THE SCHEMA BUILDS ONE.
--
-- 123's `instantiate_character` tried to copy a creature's kit in one
-- statement, inventing a `gen_random_uuid()` for each container's
-- `entity_id` so it could re-point the children at their new parents
-- before any row existed. It fails on the first creature that carries a
-- container:
--
--   insert or update on table "objects" violates foreign key constraint
--   "objects_entity_id_fkey" - Key (entity_id)=(...) is not present in
--   table "entities"
--
-- CORRECTED FORWARD rather than by editing 123, which has been applied -
-- the same rule 115 followed for 114.
--
-- ---------------------------------------------------------------------
-- WHAT 123 DID NOT KNOW ABOUT ENTITIES
-- ---------------------------------------------------------------------
--
-- An `entity_id` is not an object's identity - it is the identity of a
-- thing that can HOLD other things, and only a container has one. 030
-- gives it out from a BEFORE INSERT trigger, `container_gets_an_entity`,
-- which looks the item up, decides whether it is a container, and
-- inserts into `entities` to get a real id. An ordinary sword's
-- `entity_id` is NULL and should stay that way.
--
-- So the copy cannot know a child's new parent before the parent row
-- exists. It has to go a level at a time, taking the id the trigger
-- hands back and carrying it down - which is what this does, by
-- recursing on (source holder, new holder) rather than trying to
-- precompute the whole tree.
--
-- THE DEPTH GUARD IS THE SCHEMA'S OWN. 038 capped container nesting and
-- `no_container_cycles` enforces 16; this stops at 16 too rather than
-- inventing a second number.
--
-- AND TWO TRIGGERS DO THEIR OWN WORK ON THE WAY: a copied container
-- gets a fresh entity, and `unheld_is_unequipped` clears the slot and
-- attunement of anything whose new holder is not a character - which is
-- exactly right for a torch inside a backpack.

create or replace function public.copy_kit(
  p_src_holder uuid,
  p_dst_holder uuid,
  p_game_id    uuid,
  p_depth      int default 0
) returns void
language plpgsql
security definer
set search_path to 'public'
as $function$
declare
  o       public.objects%rowtype;
  new_ent uuid;
begin
  if p_depth >= 16 then
    raise exception 'containers nested too deep to copy';
  end if;

  for o in
    select * from public.objects where holder_id = p_src_holder
  loop
    -- NO entity_id IN THE COLUMN LIST. The trigger decides whether this
    -- thing can hold anything and mints the row in `entities` if so;
    -- supplying one here is what 123 got wrong.
    insert into public.objects (
      game_id, holder_id, item_key, quantity, name, slot,
      attuned, proficient_override, uses_spent, uses_max,
      size_override, holds_size_override, weight_override, price_override,
      damage_number_override, damage_denomination_override,
      damage_types_override, properties_override, base_ac_override, grants)
    values (
      p_game_id, p_dst_holder, o.item_key, o.quantity, o.name, o.slot,
      o.attuned, o.proficient_override, o.uses_spent, o.uses_max,
      o.size_override, o.holds_size_override, o.weight_override, o.price_override,
      o.damage_number_override, o.damage_denomination_override,
      o.damage_types_override, o.properties_override, o.base_ac_override, o.grants)
    returning entity_id into new_ent;

    -- A CONTAINER CARRIES ITS CONTENTS DOWN. Both sides must have an
    -- entity: the source to read from, the copy to write into.
    if o.entity_id is not null and new_ent is not null then
      perform public.copy_kit(o.entity_id, new_ent, p_game_id, p_depth + 1);
    end if;
  end loop;
end;
$function$;

comment on function public.copy_kit(uuid, uuid, uuid, int) is
  'Copy every object held by one entity onto another, contents and all. A level at a time, because only a container has an entity_id and the trigger that mints it runs on insert - so a child''s new parent cannot be known before the parent row exists.';

-- ---------------------------------------------------------------------
-- AND THE COPY USES IT
-- ---------------------------------------------------------------------
-- Everything above the kit is 123 verbatim; only the final statement
-- changes, from one recursive CTE to one call.
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

  -- THE KIT, AND WHAT IS INSIDE IT.
  --
  -- NOT COPIED, ON PURPOSE: spent spell slots, spent feature uses, hit
  -- point events, death saves. A creature arrives rested and whole,
  -- which is what "place a goblin" means - and a template that has been
  -- poked at in the Creatures tab does not bleed onto the creatures
  -- made from it.
  perform public.copy_kit(src.entity_id, eid, p_game_id, 0);

  return cid;
end;
$function$;

grant execute on function public.instantiate_character(uuid, uuid, text) to authenticated;
