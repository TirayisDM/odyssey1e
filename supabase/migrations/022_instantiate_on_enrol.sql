-- =====================================================================
-- 022_instantiate_on_enrol.sql
-- odyssey1e — a goblin becomes an individual the moment it is enrolled
-- =====================================================================
--
-- THE DEFECT THIS REMOVES. An actor owned almost nothing. `label`,
-- `ac_override`, `hp_override`, `initiative`, `active` and the death
-- counters were its own; ITS SCORES, ITS MAXIMUM HIT POINTS, ITS ARMOUR
-- CLASS AND ITS WEAPONS WERE READ LIVE FROM THE STATBLOCK, every time.
--
-- So editing a statblock edited every goblin that had ever come off it,
-- retroactively, in every encounter. Concretely, because hp_current is
-- derived as hp_max plus the sum of damage: raise the goblin statblock
-- from 7 to 9 and every wounded goblin in play heals two, including the
-- ones lying unconscious. Nothing warned anyone. The roll log survived
-- only because rolls snapshot target_value and target_label.
--
-- THE DECISION. Editing a goblin is an edit of THAT goblin. A Goblin
-- Elite Assassin may be a recurring type, but each one generated from it
-- is an individual and does not inherit later changes to the type.
-- Species rules may one day be inherited; that door is left open by
-- keeping npc_key on the actor as PROVENANCE, and closed for now.
--
-- WHY THIS IS THE CONVERGENCE MIGRATION AND NOT A COUSIN OF IT
--
-- An instance that owns its scores, its kit, its hit points and its
-- death saves IS a character. Everything needed already exists on
-- `characters` - level, hp_max, ac_mode, ac_override, the death
-- counters, size, the proficiency arrays - plus character_abilities and
-- character_items. Inventing actor_abilities and actor_items beside
-- them would have been a third parallel structure to keep in step, which
-- is the debt STATUS.md already lists twice.
--
-- So enrolment CREATES A CHARACTER. `npcs` stops being a live parent and
-- becomes purely a template.
--
--   npcs              the type. Scores, kit, AC, HP - a pattern.
--   characters        the individual, is_npc true. Owns everything.
--   encounter_actors  who is in this fight, pointing at a character,
--                     remembering which type it came from.
--
-- WHAT THIS COLLAPSES. hp_events and the death-save tally stop needing
-- an either/or between a character and an actor: every participant has
-- a character now. AC stops being stored-for-monsters and computed-for-
-- players - it is one mechanism, and a statblock's AC arrives as
-- ac_mode 'flat' with an override, which a DM can switch to 'default'
-- to have it computed from armour like anyone else's. Proficiency stops
-- being assumed-for-monsters: the statblock's assumption is written
-- ONCE, at instantiation, as an explicit proficient_override on each
-- copied item. The tri-state stops meaning two different things.
--
-- TWO NEW COLUMNS, BOTH EARNING THEIR PLACE
--
-- characters.is_npc is a LABEL, not a structure. It exists so a player's
-- character list does not fill with goblins, and so "save as template"
-- has something to mean later. It changes no rule.
--
-- characters.prof_bonus is a nullable OVERRIDE. A character derives the
-- bonus from level because levelling is the rule that moves it; a
-- statblock simply states one. NULL means derive, and the level formula
-- stays the only place the levelling rule is written. Sheet.prof_bonus
-- in Rust already works exactly this way - this is the column catching
-- up to it, and it deletes a divergence rather than adding one.
-- =====================================================================

-- ---------------------------------------------------------------------
-- The two columns.
-- ---------------------------------------------------------------------

alter table public.characters add column is_npc boolean not null default false;
alter table public.characters add column prof_bonus integer
  check (prof_bonus is null or prof_bonus between 0 and 9);

comment on column public.characters.is_npc is
  'A LABEL, NOT A STRUCTURE. An NPC instance is a character in every mechanical respect - that is the point of 022. This exists so a player''s character list is not full of goblins, and so "save as template" has something to mean. It changes no rule.';
comment on column public.characters.prof_bonus is
  'NULL means derive from level, which is what a player character does because levelling is the rule that moves it. A statblock STATES its bonus and has no class levels, so an instance off one carries the stated value. The level formula in Rust remains the only place the levelling rule is written.';

-- ---------------------------------------------------------------------
-- Instantiation. One function so a half-built goblin cannot exist.
-- ---------------------------------------------------------------------

create function public.instantiate_npc(
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
  -- A game-scoped statblock shadows the global one sharing its key, the
  -- precedence every reference table here uses.
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
    hp_max, ac_mode, ac_override, size)
  values (
    p_game_id,
    -- THE DM OWNS ITS MONSTERS, taken from the game rather than from
    -- auth.uid(). Only the DM can enrol, so in the app the two are the
    -- same value and the insert policy is satisfied either way - but
    -- auth.uid() is NULL inside a migration, and the backfill below runs
    -- inside one.
    (select g.dm_uid from public.games g where g.id = p_game_id),
    coalesce(nullif(btrim(p_label), ''), n.name),
    true,
    n.level,
    -- STATED, because a statblock has no class levels to derive from.
    n.prof_bonus,
    n.hp_max,
    -- The statblock's AC arrives as a flat override rather than as a
    -- second mechanism. A DM who puts real armour on this one can
    -- switch it to 'default' and have it computed like anyone else's.
    'flat',
    n.ac,
    n.size)
  returning id into cid;

  -- The seed trigger has already created six ability rows at 10; this
  -- writes the statblock's numbers over them.
  update public.character_abilities a
     set score = case a.ability::text
                   when 'str' then n.str when 'dex' then n.dex
                   when 'con' then n.con when 'int' then n.intl
                   when 'wis' then n.wis when 'cha' then n.cha
                 end
   where a.character_id = cid;

  -- The kit, with the statblock's assumption written down ONCE rather
  -- than re-read as a rule every time. 019 had npc_items read NULL as
  -- proficient; here that becomes an explicit TRUE on the instance, so
  -- the tri-state on character_items keeps its single meaning.
  -- DISTINCT ON because a kit can hold a global row AND this campaign's
  -- override for the same item, and character_items is keyed by
  -- (character_id, item_key) - two rows would collide. Game-scoped wins,
  -- the precedence every reference read here uses.
  insert into public.character_items
    (character_id, item_key, quantity, equipped, proficient_override)
  select cid, x.item_key, x.quantity, x.equipped,
         coalesce(x.proficient_override, true)
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

comment on function public.instantiate_npc(text, uuid, text) is
  'Make an individual from a type. Copies the statblock''s scores, kit, AC and hit points onto a new character so later edits to the type never reach it - see 022. Returns the character id.';

-- 017's pattern, applied on the way in.
revoke all on function public.instantiate_npc(text, uuid, text) from public, anon;
grant execute on function public.instantiate_npc(text, uuid, text) to authenticated;

-- ---------------------------------------------------------------------
-- The actors already enrolled become individuals too.
--
-- Their CURRENT hit points must not move. hp_current is hp_max plus the
-- sum of damage events, and those events point at the actor row; the new
-- character inherits the maximum and the old events keep pointing where
-- they always did, so the arithmetic lands on the same number.
-- ---------------------------------------------------------------------

-- THE XOR GOES FIRST. It reads "a character OR a statblock, never
-- both", so it refuses the backfill's very first write - the row it is
-- rejecting is one that now legitimately has both, which is the whole
-- point of 022. Dropped before the loop rather than after it.
alter table public.encounter_actors drop constraint encounter_actors_one_kind_check;

-- actor_update_scope refuses a change to character_id from anyone who
-- is not the DM, and a migration has no auth.uid() to be one. Disabled
-- for the backfill and put straight back: the guard is right, and this
-- is the one write that has to go around it.
alter table public.encounter_actors disable trigger actor_update_scope;

do $$
declare
  a record;
  cid uuid;
begin
  for a in
    select ea.id, ea.npc_key, ea.label, ea.hp_override, ea.ac_override,
           ea.death_successes, ea.death_failures, ea.dead, e.game_id, e.id as enc
      from public.encounter_actors ea
      join public.encounters e on e.id = ea.encounter_id
     where ea.character_id is null and ea.npc_key is not null
  loop
    cid := public.instantiate_npc(a.npc_key, a.game_id, a.label);

    -- Per-instance overrides already on the actor win over the type.
    update public.characters
       set hp_max          = coalesce(a.hp_override, hp_max),
           ac_override     = coalesce(a.ac_override, ac_override),
           death_successes = a.death_successes,
           death_failures  = a.death_failures,
           dead            = a.dead
     where id = cid;

    update public.encounter_actors set character_id = cid where id = a.id;
  end loop;
end $$;

alter table public.encounter_actors enable trigger actor_update_scope;

-- ---------------------------------------------------------------------
-- Every actor is a character now.
--
-- The old check was an XOR: a character OR a statblock, never both. That
-- was the shape when an NPC actor had no character behind it. Now it
-- always does, and npc_key survives alongside as provenance - which is
-- the door left open for species rules.
-- ---------------------------------------------------------------------

alter table public.encounter_actors alter column character_id set not null;

comment on column public.encounter_actors.character_id is
  'The individual in this fight. NOT NULL since 022: every participant is a character, including monsters, because an instance that owns its scores and its wounds is one. Enrolling an NPC creates this row from a statblock.';
comment on column public.encounter_actors.npc_key is
  'PROVENANCE ONLY since 022. Which statblock this individual was made from. Nothing is read through it - the character owns its scores, kit, AC and hit points - but it records the type, which is what a future species-rules layer would look through. NULL means a player character.';
