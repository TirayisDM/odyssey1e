-- =====================================================================
-- 019_npc_gear.sql
-- odyssey1e — a monster picks up a real weapon
-- =====================================================================
--
-- WHY THIS EXISTS: nothing could make an NPC act. roll_named takes a
-- character_id and reads a Sheet - ability scores, a proficiency bonus,
-- a loadout - and a goblin had none of those, so there was nothing for
-- the attack path to read. The plumbing underneath was already fine:
-- actions.character_id and rolls.character_id are both nullable, and
-- 421d08c proved an NPC roll can snapshot its own label.
--
-- THE CHOICE TAKEN, AND THE ONE NOT TAKEN
--
-- A printed statblock says "Scimitar. +4 to hit, 1d6+2 slashing", and
-- the obvious move was to store exactly that - an npc_attacks table of
-- numbers, matching how monsters are written and following the
-- precedent 011 set for npcs.ac ("stored, unlike a character's").
--
-- This does the other thing. A monster gets ability scores and carries
-- real items out of the same catalogue characters use, and every number
-- is derived by the same code that derives a character's. The cost is
-- this migration and reverse-engineering a printed +4 into scores. What
-- it buys is that everything already built applies without a second
-- implementation: equipment.rs decides proficiency, `thr` grants a
-- thrown mode, `fin` picks the better of STR and DEX, techniques attach
-- to an NPC's weapon exactly as they attach to Rodnar's, and there is
-- one attack path rather than two that must agree.
--
-- The alternative would have meant two kinds of attack in the engine
-- forever, and the first rule that differed between them would have
-- been a bug nobody could see.
--
-- WHAT A HANDAXE ACTUALLY DOES, AND WHY THE GOBLIN IS WORSE THAN THE
-- BOOK. A handaxe is lgt and thr but NOT fin, so ability_for gives it
-- STR in both melee and thrown. A Monster Manual goblin has STR 8, so
-- its handaxe is +1 to hit, not the +4 the book prints. The book's
-- goblin carries a SCIMITAR, which is finesse, so it swings on DEX 14.
-- Nothing here is wrong: that gap is the engine deriving a number
-- instead of copying one, and it is visible only because the derivation
-- is real. Give the goblin a scimitar and it hits like the book again.
--
-- PROFICIENCY IS ASSUMED FOR A MONSTER. proficient_override exists for
-- the exception, but NULL reads as TRUE on an NPC rather than "derive
-- it" - a goblin is proficient with the axe its statblock says it
-- carries, and there is no training model behind a monster to consult.
-- That is the opposite of the reading on character_items, where NULL
-- means derive, and the two are different because the question is.
-- =====================================================================

-- ---------------------------------------------------------------------
-- A statblock gets the scores every derived number comes from.
-- ---------------------------------------------------------------------

alter table public.npcs add column str integer not null default 10 check (str between 1 and 30);
alter table public.npcs add column dex integer not null default 10 check (dex between 1 and 30);
alter table public.npcs add column con integer not null default 10 check (con between 1 and 30);
alter table public.npcs add column intl integer not null default 10 check (intl between 1 and 30);
alter table public.npcs add column wis integer not null default 10 check (wis between 1 and 30);
alter table public.npcs add column cha integer not null default 10 check (cha between 1 and 30);
alter table public.npcs add column prof_bonus integer not null default 2
  check (prof_bonus between 0 and 9);
alter table public.npcs add column level integer not null default 1
  check (level between 0 and 30);

comment on column public.npcs.intl is
  'INTELLIGENCE. Named intl and not int because int is a reserved word in SQL and quoting a column name for the rest of its life is a worse trade than four letters. The engine calls it "int" like every other ability.';
comment on column public.npcs.prof_bonus is
  'STATED, not derived from level. A character computes this from their level because levelling is the rule that moves it; a monster''s is simply part of what it is, and challenge rating is not modelled here.';
comment on column public.npcs.level is
  'Only used to gate techniques by min_level. A monster has no class levels; this is the number a technique is measured against and 1 is the honest default.';

-- ---------------------------------------------------------------------
-- What a statblock carries.
-- ---------------------------------------------------------------------

create table public.npc_items (
  id                  uuid primary key default gen_random_uuid(),
  npc_key             text not null,
  item_key            text not null,
  game_id             uuid references public.games(id) on delete cascade,
  quantity            integer not null default 1 check (quantity >= 1),
  equipped            boolean not null default true,
  proficient_override boolean
);

comment on table public.npc_items is
  'The kit on a statblock, from the same catalogue characters buy from. Belongs to the STATBLOCK, not to an instance: every goblin off one row carries the same axe, the same way every goblin has the same AC. An instance that differs is a different statblock.';
comment on column public.npc_items.npc_key is
  'References npcs.key by value. See npcs.key for why not an FK.';
comment on column public.npc_items.item_key is
  'References items.key by value. Same reasoning, and the same lack of protection - nothing catches a typo but a checker in Rust.';
comment on column public.npc_items.equipped is
  'Defaults TRUE, unlike character_items. A statblock lists what the monster is fighting with; gear it is merely carrying is the rarer case and has to be said.';
comment on column public.npc_items.proficient_override is
  'The exception only. NULL reads as PROFICIENT on an NPC - a monster is proficient with what its statblock hands it, and there is no training model to derive from. character_items reads NULL as "derive", and the difference is deliberate.';

create unique index npc_items_global_idx
  on public.npc_items(npc_key, item_key) where game_id is null;
create unique index npc_items_game_idx
  on public.npc_items(npc_key, item_key, game_id) where game_id is not null;
create index npc_items_key_idx on public.npc_items(npc_key);

alter table public.npc_items enable row level security;

create policy "npc_items: read global or own game"
  on public.npc_items for select
  using (game_id is null or public.is_game_member(game_id));
create policy "npc_items: dm writes own game"
  on public.npc_items for insert
  with check (game_id is not null and public.is_game_dm(game_id));
create policy "npc_items: dm updates own game"
  on public.npc_items for update
  using (game_id is not null and public.is_game_dm(game_id))
  with check (game_id is not null and public.is_game_dm(game_id));
create policy "npc_items: dm deletes own game"
  on public.npc_items for delete
  using (game_id is not null and public.is_game_dm(game_id));

-- ---------------------------------------------------------------------
-- The axe itself. A standard simple weapon, not a campaign item, so it
-- is seeded global beside the rest of the catalogue.
-- ---------------------------------------------------------------------

insert into public.items
  (key, game_id, name, kind, base_item,
   weapon_class, damage_number, damage_denomination, damage_types, properties,
   range_reach, range_value, range_long,
   rarity, price, denom, weight)
values
  ('handaxe', null, 'Handaxe', 'weapon', 'handaxe',
   'simpleM', 1, 6, array['slashing']::text[], array['lgt','thr']::text[],
   null, 20, 60,
   null, 5, 'gp', 2);

-- ---------------------------------------------------------------------
-- The goblin, as the Monster Manual has it, and its axe.
-- ---------------------------------------------------------------------

update public.npcs
   set str = 8, dex = 14, con = 10, intl = 10, wis = 8, cha = 8,
       prof_bonus = 2, level = 1
 where key = 'goblin' and game_id is null;

insert into public.npc_items (npc_key, item_key, game_id, quantity, equipped)
values ('goblin', 'handaxe', null, 1, true);
