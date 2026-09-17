-- =====================================================================
-- 008_items.sql
-- odyssey1e — the inventory the character sheet never had
-- =====================================================================
--
-- A catalogue and a junction, following the pattern 005 set for skills
-- and 006 reused for dice: a surrogate id, a nullable game_id, and two
-- partial unique indexes doing the work a composite primary key cannot.
-- NULL game_id is global content; a set game_id is a campaign override
-- shadowing the global row with the same key.
--
-- WHAT IS HERE
--
-- items            13 rows seeded from the Foundry export in the _RAW
--                  tab of Application Data.xlsx: 3 weapons, 1 armor,
--                  and the mundane gear a priest carries.
-- character_items  what a character owns, what is equipped, what is
--                  attuned, and how many charges are spent. Character
--                  state, not reference data, but it only makes sense
--                  next to items so it lives here. Seeded empty.
-- characters       gains weapon_profs and armor_profs, without which
--                  proficiency cannot be derived at all.
--
-- FACTS ONLY. NO DERIVED NUMBERS.
--
-- The AppSheet system stored a computed Attacks tab: to-hit '+5' and
-- damage '1d6+2' per weapon, with the character's ability modifier and
-- proficiency bonus already folded in. That is why its own header told
-- you to reseed after every level-up - the numbers belonged to the
-- character, not the weapon, and went stale the moment either changed.
--
-- This table stores 1d6 and nothing else. To-hit and damage are
-- computed in Rust from the live sheet, per the rule that rules live in
-- Rust and facts live in Postgres. Finesse alone settles it: the
-- ability a weapon uses depends on which of STR and DEX is higher
-- today, so any stored value is a guess with a shelf life.
--
-- TRAPS PAID FOR WHILE WRITING THIS
--
-- Foundry spells armor two ways. The item subtype is light/medium/
-- heavy/shield; the character's armorProf is lgt/med/hvy/shl. Matching
-- one against the other fails silently and every armor proficiency
-- check comes back false. armor_category is normalized to the prof
-- spelling on the way in, so the two vocabularies meet here and only
-- here.
--
-- A weapon carries a meaningless armor.value. The Heavy Crossbow's
-- export says armor: {value: 10} - a Foundry default, not an AC. Only
-- rows where kind = 'armor' take base_ac; the rest are null on purpose.
--
-- uses.max is a STRING in the export ('7', '1', ''), not a number.
-- Empty means no limit, which is not the same as zero.
--
-- WHAT IS DELIBERATELY NOT HERE
--
-- No AC calculation. base_ac and dex_cap are stored because Scale Mail
-- has to exist as a row anyway, but nothing computes an armor class -
-- there is no combat loop to consume one, and the Character tab already
-- carries AC_Flat if a sheet wants to show a number.
--
-- No containers, encumbrance or currency. The Priest's Pack is seeded
-- as an ordinary item; what it contains is not modelled. weight and
-- price are stored because they are scalar facts about an item and one
-- column each - a system that reads them is a different thing entirely,
-- and is not being built here.
--
-- No descriptions. The source carries HTML with entities and Foundry
-- @UUID references that would need resolving. Recoverable from _RAW
-- whenever something wants to render them.
--
-- ITEM KEYS ARE LOAD-BEARING. 007 already points techniques at
-- mace_of_the_deep_song, light_hammer and heavy_crossbow. There is no
-- FK to catch a typo - by design, see items.key below - so equipment.rs
-- should assert every distinct techniques.item_key resolves here.
-- Keys are the display name lowercased, apostrophes dropped, every run
-- of non-alphanumerics collapsed to one underscore.
-- =====================================================================

create table public.items (
  id                   uuid primary key default gen_random_uuid(),
  key                  text not null,
  game_id              uuid references public.games(id) on delete cascade,
  name                 text not null,
  kind                 text not null
                         check (kind in ('weapon','armor','equipment',
                                         'consumable','loot','container')),
  base_item            text,

  -- weapon facts; null on everything that is not a weapon
  weapon_class         text check (weapon_class in ('simpleM','simpleR',
                                                    'martialM','martialR')),
  damage_number        integer check (damage_number is null or damage_number >= 1),
  damage_denomination  integer check (damage_denomination is null or damage_denomination >= 2),
  damage_types         text[] not null default '{}',
  properties           text[] not null default '{}',
  range_reach          integer,
  range_value          integer,
  range_long           integer,

  -- armor facts; null on everything that is not armor
  armor_category       text check (armor_category in ('lgt','med','hvy','shl')),
  base_ac              integer,
  dex_cap              integer,

  -- common
  rarity               text,
  price                integer,
  denom                text check (denom in ('cp','sp','ep','gp','pp')),
  weight               numeric,
  image_url            text,
  description          text
);

comment on table public.items is
  'The item catalogue. Facts about things, never numbers derived from whoever is holding them. Seeded from the Foundry export that the AppSheet Inventory and Attacks tabs were both generated from.';
comment on column public.items.key is
  'Stable slug. Referenced by value from techniques.item_key and character_items.item_key, never by FK - a global item and a game override share a key, so key is unique only within each partition and a partial unique index cannot back a foreign key. Same reasoning as dice_faces.set_key.';
comment on column public.items.game_id is
  'NULL means global. Set means this campaign only.';
comment on column public.items.kind is
  'What the item is, for deciding which fact columns apply. Armor is Foundry type equipment with an armor subtype, collapsed to its own kind here because nothing else about it behaves like equipment.';
comment on column public.items.base_item is
  'Foundry baseItem: mace, lighthammer, heavycrossbow, scalemail. The weapon or armor TYPE, as opposed to this specific instance. Proficiency can be granted by base_item as well as by class, and it is the likely key if techniques ever attach to weapon families rather than to one weapon.';
comment on column public.items.weapon_class is
  'simpleM simpleR martialM martialR. The M/R suffix is melee or ranged and decides the default ability when no property overrides it. Matched against characters.weapon_profs by its sim/mar prefix.';
comment on column public.items.damage_number is
  'Dice COUNT only. The ability modifier is not here and must never be - that is the baking the Attacks tab did.';
comment on column public.items.damage_denomination is
  'Die faces. House techniques use 7 and 14, which the dice engine already accepts.';
comment on column public.items.properties is
  'Foundry property codes: lgt thr fin ver two hvy rch amm lod ret mgc sil foc, plus stealthDisadvantage on armor. fin and thr are read as rules - finesse picks the better of STR and DEX, thrown grants a second attack mode - so this column is engine input, not display text.';
comment on column public.items.range_reach is
  'Melee reach in feet. The Mace is 8, not the default 5.';
comment on column public.items.armor_category is
  'lgt med hvy shl - NORMALIZED from the export''s light/medium/heavy/shield so it can be matched directly against characters.armor_profs. The two vocabularies are reconciled here and nowhere else.';
comment on column public.items.base_ac is
  'Armor only. A weapon''s armor.value in the export is a Foundry default and is discarded.';
comment on column public.items.dex_cap is
  'Maximum DEX modifier this armor allows. 2 for medium. Stored unused until something computes AC.';
comment on column public.items.weight is
  'Scalar fact. No encumbrance rule reads it.';

create unique index items_global_key_idx
  on public.items(key) where game_id is null;
create unique index items_game_key_idx
  on public.items(key, game_id) where game_id is not null;
create index items_kind_idx on public.items(kind);

alter table public.items enable row level security;

create policy "items: read global or own game"
  on public.items for select
  using (game_id is null or public.is_game_member(game_id));
create policy "items: dm writes own game"
  on public.items for insert
  with check (game_id is not null and public.is_game_dm(game_id));
create policy "items: dm updates own game"
  on public.items for update
  using (game_id is not null and public.is_game_dm(game_id))
  with check (game_id is not null and public.is_game_dm(game_id));
create policy "items: dm deletes own game"
  on public.items for delete
  using (game_id is not null and public.is_game_dm(game_id));

-- ---------------------------------------------------------------------
-- Seed: 13 items, generated from the _RAW Foundry export.
-- ---------------------------------------------------------------------

insert into public.items
  (key, game_id, name, kind, base_item,
   weapon_class, damage_number, damage_denomination, damage_types, properties,
   range_reach, range_value, range_long,
   armor_category, base_ac, dex_cap,
   rarity, price, denom, weight, image_url)
values
  ('priests_pack', null, 'Priest''s Pack', 'container', null, null, null, null, '{}', '{}', null, null, null, null, null, null, null, 33, 'gp', 5, 'https://assets.forge-vtt.com/bazaar/core/icons/containers/bags/pack-engraved-leather-blue.webp'),
  ('blanket', null, 'Blanket', 'loot', null, null, null, null, '{}', '{}', null, null, null, null, null, null, null, 5, 'sp', 3, 'https://assets.forge-vtt.com/bazaar/core/icons/sundries/survival/bedroll-pink.webp'),
  ('rations', null, 'Rations', 'consumable', null, null, null, null, '{}', '{}', null, null, null, null, null, null, null, 5, 'sp', 2, 'https://assets.forge-vtt.com/bazaar/core/icons/consumables/food/berries-ration-round-red.webp'),
  ('lamp', null, 'Lamp', 'equipment', null, null, null, null, '{}', '{}', null, null, null, null, null, null, null, 5, 'sp', 1, 'https://assets.forge-vtt.com/bazaar/core/icons/sundries/lights/lantern-iron-yellow.webp'),
  ('tinderbox', null, 'Tinderbox', 'equipment', null, null, null, null, '{}', '{}', null, null, null, null, null, null, null, 5, 'sp', 1, 'https://assets.forge-vtt.com/bazaar/core/icons/sundries/lights/torch-black.webp'),
  ('robe', null, 'Robe', 'equipment', null, null, null, null, '{}', '{}', null, null, null, null, null, null, null, 1, 'gp', 4, 'https://assets.forge-vtt.com/bazaar/core/icons/equipment/back/mantle-collared-black.webp'),
  ('holy_water', null, 'Holy Water', 'consumable', null, null, null, null, '{}', '{}', null, null, null, null, null, null, null, 25, 'gp', 1, 'https://assets.forge-vtt.com/bazaar/core/icons/consumables/potions/bottle-round-empty-glass.webp'),
  ('clothes_fine', null, 'Clothes, Fine', 'equipment', null, null, null, null, '{}', '{}', null, null, null, null, null, null, null, 15, 'gp', 6, 'https://assets.forge-vtt.com/bazaar/core/icons/equipment/back/cloak-collared-feathers-green.webp'),
  ('scale_mail', null, 'Scale Mail', 'armor', 'scalemail', null, null, null, '{}', array['stealthDisadvantage']::text[], null, null, null, 'med', 14, 2, null, 50, 'gp', 45, 'https://assets.forge-vtt.com/bazaar/core/icons/equipment/chest/breastplate-banded-steel.webp'),
  ('mace_of_the_deep_song', null, 'Mace of the Deep Song', 'weapon', 'mace', 'simpleM', 1, 6, array['bludgeoning']::text[], '{}', 8, null, null, null, null, null, null, 70, 'sp', 14, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Mace%20of%20the%20Deep%20Song.webp'),
  ('light_hammer', null, 'Light Hammer', 'weapon', 'lighthammer', 'simpleM', 1, 4, array['bludgeoning']::text[], array['lgt','thr']::text[], null, 20, 60, null, null, null, null, 2, 'gp', 2, 'https://assets.forge-vtt.com/bazaar/core/icons/weapons/hammers/shorthammer-double-stone-engraved.webp'),
  ('the_ember', null, 'The Ember', 'equipment', null, null, null, null, '{}', array['foc','mgc']::text[], null, null, null, null, null, null, 'rare', 4000, 'gp', 0, 'https://assets.forge-vtt.com/bazaar/core/icons/weapons/staves/staff-obsidian.webp'),
  ('heavy_crossbow', null, 'Heavy Crossbow', 'weapon', 'heavycrossbow', 'martialR', 1, 10, array['piercing']::text[], array['amm','hvy','lod','two']::text[], null, 100, 400, null, null, null, null, 50, 'gp', 18, 'https://assets.forge-vtt.com/bazaar/core/icons/weapons/crossbows/crossbow-loaded-black.webp');

-- ---------------------------------------------------------------------
-- character_items: what a character actually has.
-- ---------------------------------------------------------------------

create table public.character_items (
  character_id         uuid not null references public.characters(id) on delete cascade,
  item_key             text not null,
  quantity             integer not null default 1 check (quantity >= 0),
  equipped             boolean not null default false,
  attuned              boolean not null default false,
  proficient_override  boolean,
  uses_spent           integer not null default 0 check (uses_spent >= 0),
  uses_max             integer check (uses_max is null or uses_max >= 0),
  acquired_at          timestamptz not null default now(),
  primary key (character_id, item_key)
);

comment on table public.character_items is
  'Which items a character owns and their per-character state. The Inventory tab, minus the derived Attacks tab it fed. Seeded empty: character ids are runtime data, so rows are created by the app, not here.';
comment on column public.character_items.item_key is
  'References items.key by value. See items.key for why not a FK.';
comment on column public.character_items.quantity is
  'Stacks. Seven rations are one row with quantity 7, not seven rows. Two of the same item in different states would need two catalogue entries, which is correct - a +1 sword is not a sword.';
comment on column public.character_items.equipped is
  'In hand or worn. Several weapons may be equipped at once, so this is NOT constrained the way character_dice.equipped is. At most one ARMOR should be equipped, but that rule needs items.kind, which lives in another table and cannot be reached from a partial index - so it is enforced in equipment.rs, not here. A schema that cannot state a rule should say so rather than pretend.';
comment on column public.character_items.proficient_override is
  'TRI-STATE, and the nullability is the point. TRUE or FALSE is an explicit answer from the source (the Mace ships proficient: 1). NULL means derive it from characters.weapon_profs, which is how the Light Hammer earns proficiency - it is not flagged, it matches sim. A boolean not null default false here would silently strip proficiency from every unflagged weapon.';
comment on column public.character_items.uses_spent is
  'Charges consumed. The Ember ships 6 of 7 spent.';
comment on column public.character_items.uses_max is
  'Charge capacity, or NULL for no limit. The export stores this as a string and uses empty for unlimited, which is not the same as zero.';
comment on column public.character_items.acquired_at is
  'Kept for ordering an inventory view. Same reason character_dice has it.';

create index character_items_equipped_idx
  on public.character_items(character_id) where equipped;

alter table public.character_items enable row level security;

create policy "character_items: read with character"
  on public.character_items for select
  using (exists (select 1 from public.characters c
                 where c.id = character_id and public.is_game_member(c.game_id)));
create policy "character_items: owner or dm writes"
  on public.character_items for insert
  with check (exists (select 1 from public.characters c
                      where c.id = character_id
                        and (c.owner_uid = auth.uid() or public.is_game_dm(c.game_id))));
create policy "character_items: owner or dm updates"
  on public.character_items for update
  using (exists (select 1 from public.characters c
                 where c.id = character_id
                   and (c.owner_uid = auth.uid() or public.is_game_dm(c.game_id))))
  with check (exists (select 1 from public.characters c
                      where c.id = character_id
                        and (c.owner_uid = auth.uid() or public.is_game_dm(c.game_id))));
create policy "character_items: owner or dm deletes"
  on public.character_items for delete
  using (exists (select 1 from public.characters c
                 where c.id = character_id
                   and (c.owner_uid = auth.uid() or public.is_game_dm(c.game_id))));

-- ---------------------------------------------------------------------
-- The character gains its proficiencies.
--
-- Without these two columns nothing can be derived: every unflagged
-- weapon falls back to proficient_override IS NULL and has nothing to
-- match against. They default to empty rather than to a plausible set
-- because a wrong proficiency is worse than an absent one - it changes
-- to-hit silently.
-- ---------------------------------------------------------------------

alter table public.characters add column weapon_profs text[] not null default '{}';
alter table public.characters add column armor_profs  text[] not null default '{}';

comment on column public.characters.weapon_profs is
  'Foundry traits.weaponProf.value. sim and mar are whole classes; a bare baseItem (mace, lighthammer) grants one weapon. Rodnar holds only sim, which is exactly why his Heavy Crossbow is not proficient and his to-hit with it is DEX alone.';
comment on column public.characters.armor_profs is
  'Foundry traits.armorProf.value: lgt med hvy shl. Matched against items.armor_category, which 008 normalizes to this same spelling.';

-- Backfill the two test characters described in STATUS.md - the
-- deliberate twins, both built from Rodnar. Matched by name, not by a
-- generated id. A no-op on any database that does not have them.
update public.characters
   set weapon_profs = array['sim']::text[],
       armor_profs  = array['lgt','med','shl']::text[]
 where name in ('Character1', 'Character2');
