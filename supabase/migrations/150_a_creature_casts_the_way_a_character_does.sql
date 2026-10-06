-- 150. A CREATURE CASTS THE WAY A CHARACTER DOES.
--
-- Spellcasting was the last of the four things 147 said the bestiary
-- could not express, and the only one with a whole subsystem already
-- built: 101 to 107, `prayers.rs`, `spellcast.rs`, a 108-spell
-- catalogue, slots that can be spent and a tab to spend them from.
--
-- None of it could be reached by a monster. Six commands found a cleric
-- class on the sheet and refused anybody else, and a monster has no
-- classes at all - so a Lich, which is an 18th-level spellcaster in its
-- own statblock, could not cast a cantrip.
--
-- ---------------------------------------------------------------------
-- THE ANSWER WAS NOT A SECOND MECHANISM
-- ---------------------------------------------------------------------
--
-- The obvious build was `npcs.caster_level` and `npcs.casting_ability`
-- and a creature-shaped path beside the character-shaped one. That is
-- the shape this codebase has regretted every single time: 022 deleted
-- ninety lines of monster sheet-building and left `load_actor_sheet` at
-- fifteen, because "a monster's sheet is loaded by the function that
-- loads anyone's".
--
-- Dave said it plainly - creatures and NPCs should work the way PCs do -
-- and the right move follows: A CREATURE THAT CASTS HAS CLASS LEVELS.
-- `character_classes` is what a player character has, `sheet.classes` is
-- what reads it, and `prayers::caster` finds a casting class there
-- without caring what kind of thing it belongs to. No creature branch,
-- anywhere, in any of it.
--
-- `npcs.class_key` has existed since 064 and was set on exactly ONE
-- statblock. What was missing is a LEVEL to go with it, and
-- `instantiate_npc` writing the class row instead of only copying the
-- key onto the character.
--
-- ---------------------------------------------------------------------
-- WHY CLASS LEVEL IS NOT `npcs.level`
-- ---------------------------------------------------------------------
--
-- 147 set `npcs.level` to the challenge rating, and CR is not caster
-- level. The book prints both and they differ:
--
--   Priest         CR 2    5th-level spellcaster
--   Cult Fanatic   CR 2    4th
--   Mage           CR 6    9th
--   Lich           CR 21   18th
--
-- Reusing `level` would give the Priest second-level slots instead of
-- fifth and take Spirit Guardians off it, which is the spell that makes
-- a priest a priest. So one column, and it buys the whole player path
-- rather than a parallel one.
--
-- ---------------------------------------------------------------------
-- A CLERIC AND A WIZARD DIFFER, AND THE DIFFERENCE IS THE SOURCE
-- ---------------------------------------------------------------------
--
-- 101-107 built one shape and called it prayers, and the name hid the
-- question Dave asked: a cleric draws from the WHOLE cleric list every
-- day and prepares a subset; a wizard may only prepare what is written
-- in a book. Same slots, same preparing, different source - and every
-- other difference follows from that one.
--
-- `prayers::Source` is the vocabulary - `whole_list` or `book` - and
-- `prayers::casts` is the table of which class casts on which ability
-- from which source. A RULE RATHER THAN A COLUMN, because `classes` has
-- never said anything about spellcasting and this is 5e's own table.
--
-- SO `prepared` CANNOT BE A BOOLEAN ANY MORE. It is carrying two
-- meanings today - true means prepared, false means cantrip, and every
-- false row in the database is a level-0 spell - and a wizard needs a
-- third value for a spell that is in the book and not prepared today.
-- One column, three states, which is what it was always describing.
--
-- ---------------------------------------------------------------------
-- WHAT IS DELIBERATELY NOT HERE
-- ---------------------------------------------------------------------
--
-- THE KNOWN CASTERS. Bard, sorcerer and warlock have a fixed list and
-- never prepare anything - a third shape, absent from `prayers::casts`
-- rather than listed and quietly treated as clerics.
--
-- THE HALF-CASTERS. Paladin and ranger prepare from a whole list on a
-- half slot table, and `slots_at` is the full table and says so.
--
-- THE WIZARD'S ACQUISITION PATH. A player wizard's book grows by two
-- spells a level and by copying scrolls and books they FIND, which is
-- the objects system rather than this one. A creature's book is what
-- its statblock says, which needs none of that.
--
-- AND THE WIZARD LIST ITSELF. The catalogue is the cleric list plus
-- Fireball and Aura of Life - 101 to 105 - so the Mage and the Lich get
-- the handful of their spells that happen to exist. That is a data gap
-- with a known shape, not a missing mechanism.

-- --- a statblock may have a class, at a level --------------------------

alter table public.npcs
  add column if not exists class_level integer not null default 0;

alter table public.npcs
  drop constraint if exists npcs_class_level_check;
alter table public.npcs
  add constraint npcs_class_level_check
  check (class_level >= 0 and class_level <= 20);

comment on column public.npcs.class_level is
  'The level of the class in class_key, when this statblock has one (150). NOT `level`, which is the challenge rating - the book prints both and they differ: a Priest is CR 2 and a 5th-level spellcaster. Zero means the class_key is a label only. instantiate_npc writes this into character_classes, so a creature that casts does it through the same class rows a player character has.';

-- --- a spell is held in one of three states ---------------------------

alter table public.character_prayers
  add column if not exists state text;

update public.character_prayers
   set state = case when prepared then 'prepared' else 'cantrip' end
 where state is null;

alter table public.character_prayers
  alter column state set not null,
  alter column state set default 'prepared';

alter table public.character_prayers
  drop constraint if exists character_prayers_state_check;
alter table public.character_prayers
  add constraint character_prayers_state_check
  check (state in ('cantrip', 'book', 'prepared'));

alter table public.character_prayers drop column if exists prepared;

comment on column public.character_prayers.state is
  'How this caster holds this spell (150). `cantrip` needs no slot and is never prepared; `prepared` can be cast today; `book` is known and not prepared - a wizard''s book, and the state a boolean could not express. Before 150 this was a boolean carrying two meanings at once: true for prepared and false for cantrip, with every false row a level-0 spell.';

-- --- and a statblock may know some ------------------------------------

create table if not exists public.npc_spells (
  id         uuid primary key default gen_random_uuid(),
  npc_key    text not null,
  spell_key  text not null,
  game_id    uuid references public.games(id) on delete cascade,
  state      text not null default 'prepared'
             check (state in ('cantrip', 'book', 'prepared'))
);

comment on table public.npc_spells is
  'What a statblock knows, in the shape npc_items has for its kit (150): a pattern that instantiate_npc copies onto the character it makes. A creature''s book and its prepared list are the same set unless a DM says otherwise - a monster statblock prints what it has prepared and nothing about what it chose not to.';

create unique index if not exists npc_spells_global_idx
  on public.npc_spells(npc_key, spell_key) where game_id is null;
create unique index if not exists npc_spells_game_idx
  on public.npc_spells(npc_key, spell_key, game_id) where game_id is not null;

alter table public.npc_spells enable row level security;

create policy "npc_spells: read global or own game"
  on public.npc_spells for select
  using (game_id is null or public.is_game_member(game_id));
create policy "npc_spells: dm writes own game"
  on public.npc_spells for insert
  with check (game_id is not null and public.is_game_dm(game_id));
create policy "npc_spells: dm updates own game"
  on public.npc_spells for update
  using (game_id is not null and public.is_game_dm(game_id))
  with check (game_id is not null and public.is_game_dm(game_id));
create policy "npc_spells: dm deletes own game"
  on public.npc_spells for delete
  using (game_id is not null and public.is_game_dm(game_id));

-- --- instantiate_npc gives it the class and the spells -----------------
--
-- THE SIGNATURE IS BYTE-FOR-BYTE 124's, for the third time: 095 had to
-- drop a duplicate that a parameter reorder created, because PostgREST
-- dispatches an RPC on parameter NAMES and `create or replace` with the
-- arguments in a different order makes a SECOND function rather than
-- replacing the first. The additions are the two inserts at the end.

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
         -- 137. PROFICIENT UNLESS THE STATBLOCK SAYS OTHERWISE.
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
  -- Only when the statblock states a level: `class_key` alone has been
  -- a label since 064 and giving every labelled creature class features
  -- it was never designed with would be a balance change arriving as a
  -- side effect, which is 129's mistake.
  if n.class_key is not null and n.class_level > 0 then
    insert into public.character_classes (character_id, class_key, level)
    values (cid, n.class_key, n.class_level);
  end if;

  -- 150. AND WHAT IT KNOWS. Same shape as the kit above: a pattern on
  -- the statblock, copied onto the individual, so editing the type
  -- never reaches a creature already made from it.
  insert into public.character_prayers (character_id, spell_key, state)
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
