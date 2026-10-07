-- 152. A PRAYER IS IN A STATE, NOT A BOOLEAN.
--
-- 150 taught `instantiate_character` to carry a creature's prayers over
-- when it is placed, which is right and was missing. It copied them
-- through a column that does not exist:
--
--   insert into public.character_prayers (character_id, spell_key, prepared)
--   select cid, p.spell_key, p.prepared
--
-- `character_prayers` has never had a `prepared` column. It has `state`,
-- text, NOT NULL, defaulting to 'prepared' - and 107 gave it three
-- values, not two: 'cantrip', 'prepared' and 'book'. A cantrip is KNOWN
-- rather than prepared and a spell in the book is neither, which is why
-- `prayers::may_reach` accepts 'book' and 'prepared' and refuses to
-- prepare a cantrip at all.
--
-- ---------------------------------------------------------------------
-- WHAT IT ACTUALLY BROKE, WHICH IS MORE THAN PRAYERS
-- ---------------------------------------------------------------------
--
-- EVERY ENROL. plpgsql resolves a statement's columns the first time it
-- runs, not when the function is created, so this did not fail at
-- migration time - it failed the next time anybody placed a creature.
-- `instantiate_character` is the whole of 123's "placing one is a copy",
-- so enrolling a Giant Wasp into an encounter raised
--
--   column "prepared" of relation "character_prayers" does not exist
--
-- and rolled the entire placement back. Not the prayers - the placement.
-- A creature with no prayers at all could not be brought in either,
-- because the statement is parsed whether or not it has rows to copy.
--
-- THAT IS THE COST OF A WRITE PATH WITH NO TEST, and it is the shape the
-- architecture line predicts: `src/*.rs` is tested and `src/commands/*`
-- and the SQL functions are not, so a column name that is simply wrong
-- has nothing standing between it and the user. The counter-pressure is
-- to run the thing once against the live database, which is what found
-- this - a minute after it would have found it on its own.
--
-- ---------------------------------------------------------------------
-- 'book' IS THE HONEST DEFAULT AND IT IS NOT USED HERE
-- ---------------------------------------------------------------------
--
-- The copy carries `state` across verbatim rather than deciding anything.
-- A creature that had a cantrip has a cantrip; one holding a spell in
-- the book still holds it there. Collapsing the three states to a
-- boolean and back would silently turn every cantrip into something the
-- rules do not have a word for, and there are four such rows live right
-- now.
--
-- THE SAME MISTAKE WAS IN THE RUST, in both directions - `export_creature`
-- selected `spell_key,prepared` and `import_creature` wrote `prepared` -
-- so a creature could not be written to a file or read back from one
-- either. Both now use `state`, `creature_io::Prayer` carries the word
-- rather than a flag, and `creature_io::STATES` is the list vet checks
-- against. Four tests cover the round trip, since that part is a rule
-- and rules are tested.

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
  insert into public.character_prayers (character_id, spell_key, state)
  select cid, p.spell_key, p.state
    from public.character_prayers p where p.character_id = p_source
  on conflict (character_id, spell_key) do nothing;

  perform public.copy_kit(src.entity_id, eid, p_game_id, 0);

  return cid;
end;
$function$;

grant execute on function public.instantiate_character(uuid, uuid, text) to authenticated;
