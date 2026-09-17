-- =====================================================================
-- 006_reference_data.sql
-- odyssey1e — the rest of the reference content from the AppSheet era
-- =====================================================================
--
-- Six catalogue tables and one junction, all following the pattern 005
-- established for public.skills: a surrogate id, a nullable game_id,
-- and two partial unique indexes doing the work a composite primary key
-- cannot. NULL game_id is global content every campaign shares; a set
-- game_id is a campaign-only override that shadows the global row with
-- the same key. Nothing in a client can write a global row — shared
-- content is seeded here and only here.
--
-- WHAT IS AND IS NOT IN THIS FILE
--
-- narrative_lines  360 canned lines, 18 skills x 10 x 2 packs. The
--                  asset the whole port was partly about preserving.
-- dice_sets        4 sets, and dice_faces, 80 d20 faces with art URLs.
-- character_dice   the ActorDice junction: which sets a character owns
--                  and which one is equipped. Character state, not
--                  reference data, but it only makes sense next to
--                  dice_sets so it lives here.
-- skill_prompts    34 prompt seeds for the AI narrative generator:
--                  which facts about the character the prompt should
--                  mention and which gear tags are relevant.
-- spells           24 rows from the Spellbook tab.
-- techniques       31 rows: weapon techniques with custom crit and
--                  fumble ranges.
--
-- SPELLS AND TECHNIQUES ARE PORTED AS-IS, NOT CLEANED. The Spellbook
-- and Techniques tabs were built for one character. Some values that
-- ought to be derived are baked in: a spell attack of +7, a DC of 15,
-- "2d8+4" with the casting modifier already folded into the dice. They
-- are seeded faithfully as global rows because that is what the old
-- system shipped, and the spell-slot and technique rules modules
-- (still to be ported) are where those numbers get replaced by engine
-- arithmetic. The columns carrying baked values say so in their
-- COMMENT. The Spellbook tab's Prepared column is NOT ported: prepared
-- is per-character state and belongs on a character_spells table when
-- the spell module arrives.
--
-- ODD DICE. Techniques include 1d7 and 1d14. Those are not typos in the
-- source; they are house dice and the engine must roll them. Kept.
--
-- HOW A CHARACTER PICKS A NARRATIVE PACK. characters.narrative_pack,
-- added below, names the pack. Default 'base'. The second pack is named
-- after the character it was written for; a new character can point at
-- it or at any pack a DM adds as game-scoped rows.
-- =====================================================================


-- =====================================================================
-- NARRATIVE LINES
-- =====================================================================

create table public.narrative_lines (
  id        uuid primary key default gen_random_uuid(),
  pack      text not null,
  key       text not null,
  seq       integer not null check (seq >= 1),
  game_id   uuid references public.games(id) on delete cascade,
  line      text not null,
  enabled   boolean not null default true
);

comment on table public.narrative_lines is
  'Canned prose for a roll card, chosen at random from the lines matching (pack, key). game_id NULL is global; set is a campaign override. From NarrativePacks.js v1.0.';
comment on column public.narrative_lines.pack is
  'Pack name. ''base'' is the neutral pack; other packs are voiced for a specific character or campaign. characters.narrative_pack selects one.';
comment on column public.narrative_lines.key is
  'What the line describes: a skill key (acr, ani, ...) today; save, check and attack keys as packs grow. Matches rolls.request vocabulary.';
comment on column public.narrative_lines.seq is
  'Position within (pack, key), 1-based. Only meaning is identity for the unique index and a stable order when editing.';
comment on column public.narrative_lines.game_id is
  'NULL means global. Set means this campaign only.';
comment on column public.narrative_lines.line is
  'The prose. Written in third person plural (they/their) so it needs no pronoun substitution.';
comment on column public.narrative_lines.enabled is
  'False takes a line out of rotation without deleting it.';

create unique index narrative_lines_global_idx
  on public.narrative_lines(pack, key, seq) where game_id is null;
create unique index narrative_lines_game_idx
  on public.narrative_lines(pack, key, seq, game_id) where game_id is not null;
create index narrative_lines_lookup_idx
  on public.narrative_lines(pack, key) where enabled;

alter table public.narrative_lines enable row level security;

create policy "narrative_lines: read global or own game"
  on public.narrative_lines for select
  using (game_id is null or public.is_game_member(game_id));
create policy "narrative_lines: dm writes own game"
  on public.narrative_lines for insert
  with check (game_id is not null and public.is_game_dm(game_id));
create policy "narrative_lines: dm updates own game"
  on public.narrative_lines for update
  using (game_id is not null and public.is_game_dm(game_id))
  with check (game_id is not null and public.is_game_dm(game_id));
create policy "narrative_lines: dm deletes own game"
  on public.narrative_lines for delete
  using (game_id is not null and public.is_game_dm(game_id));

alter table public.characters
  add column narrative_pack text not null default 'base';

comment on column public.characters.narrative_pack is
  'Which narrative_lines pack voices this character''s roll cards. Text by value, not a FK — packs are a naming convention, not a table. Falls back to ''base'' in the engine if the pack has no line for a key.';


-- =====================================================================
-- DICE SETS AND FACES
-- =====================================================================

create table public.dice_sets (
  id           uuid primary key default gen_random_uuid(),
  key          text not null,
  game_id      uuid references public.games(id) on delete cascade,
  name         text not null,
  price        integer not null default 0,
  preview_url  text
);

comment on table public.dice_sets is
  'A themed set of die art. rolls.dice_set_id stores key by value so a retired set never rewrites history.';
comment on column public.dice_sets.key is
  'Three-digit text code from the AppSheet era: 001 Blue Crystal, 002 Gold, 003 Midnight, 004 Red Crystal. Text so leading zeros survive.';
comment on column public.dice_sets.game_id is
  'NULL means global. Set means this campaign only.';
comment on column public.dice_sets.price is
  'For an eventual dice shop. Zero means free. Currency undefined until the shop exists.';
comment on column public.dice_sets.preview_url is
  'One representative face, usually the 20, for a picker.';

create unique index dice_sets_global_key_idx
  on public.dice_sets(key) where game_id is null;
create unique index dice_sets_game_key_idx
  on public.dice_sets(key, game_id) where game_id is not null;

alter table public.dice_sets enable row level security;

create policy "dice_sets: read global or own game"
  on public.dice_sets for select
  using (game_id is null or public.is_game_member(game_id));
create policy "dice_sets: dm writes own game"
  on public.dice_sets for insert
  with check (game_id is not null and public.is_game_dm(game_id));
create policy "dice_sets: dm updates own game"
  on public.dice_sets for update
  using (game_id is not null and public.is_game_dm(game_id))
  with check (game_id is not null and public.is_game_dm(game_id));
create policy "dice_sets: dm deletes own game"
  on public.dice_sets for delete
  using (game_id is not null and public.is_game_dm(game_id));


create table public.dice_faces (
  id         uuid primary key default gen_random_uuid(),
  set_key    text not null,
  game_id    uuid references public.games(id) on delete cascade,
  die        text not null,
  face       integer not null check (face >= 1),
  image_url  text not null
);

comment on table public.dice_faces is
  'One image per face per die per set. 80 rows today: 4 sets x d20 x 20 faces. rolls.die_image_url is snapshotted from here at resolve time, never joined at read time.';
comment on column public.dice_faces.set_key is
  'References dice_sets.key by value, not by FK — a global set and a game override share a key, so an FK could not point at one of them unambiguously. Same reasoning as character_skills.skill_key.';
comment on column public.dice_faces.game_id is
  'NULL means global. Set means this campaign only.';
comment on column public.dice_faces.die is
  'Die type as the engine names it: d20 today; d4 d6 d8 d10 d12 d100 when art exists.';
comment on column public.dice_faces.face is
  'The face value. 1..20 for a d20.';

create unique index dice_faces_global_idx
  on public.dice_faces(set_key, die, face) where game_id is null;
create unique index dice_faces_game_idx
  on public.dice_faces(set_key, die, face, game_id) where game_id is not null;

alter table public.dice_faces enable row level security;

create policy "dice_faces: read global or own game"
  on public.dice_faces for select
  using (game_id is null or public.is_game_member(game_id));
create policy "dice_faces: dm writes own game"
  on public.dice_faces for insert
  with check (game_id is not null and public.is_game_dm(game_id));
create policy "dice_faces: dm updates own game"
  on public.dice_faces for update
  using (game_id is not null and public.is_game_dm(game_id))
  with check (game_id is not null and public.is_game_dm(game_id));
create policy "dice_faces: dm deletes own game"
  on public.dice_faces for delete
  using (game_id is not null and public.is_game_dm(game_id));


-- =====================================================================
-- CHARACTER DICE  (the ActorDice junction — character state)
-- =====================================================================

create table public.character_dice (
  character_id  uuid not null references public.characters(id) on delete cascade,
  set_key       text not null,
  owned         boolean not null default true,
  equipped      boolean not null default false,
  acquired_at   timestamptz not null default now(),
  primary key (character_id, set_key)
);

comment on table public.character_dice is
  'Which dice sets a character owns and which one is in hand. Was the ActorDice tab (OwnID = CharID_SetID). A character with no rows rolls with no die art, and the engine treats that as fine.';
comment on column public.character_dice.set_key is
  'References dice_sets.key by value. See dice_faces.set_key for why not a FK.';
comment on column public.character_dice.owned is
  'False keeps the row (and acquired_at) after a set is sold or lost.';
comment on column public.character_dice.equipped is
  'At most one true per character, enforced by the partial unique index below.';

create unique index character_dice_one_equipped_idx
  on public.character_dice(character_id) where equipped;

alter table public.character_dice enable row level security;

create policy "character_dice: read with character"
  on public.character_dice for select
  using (exists (select 1 from public.characters c
                 where c.id = character_id and public.is_game_member(c.game_id)));
create policy "character_dice: owner or dm writes"
  on public.character_dice for insert
  with check (exists (select 1 from public.characters c
                      where c.id = character_id
                        and (c.owner_uid = auth.uid() or public.is_game_dm(c.game_id))));
create policy "character_dice: owner or dm updates"
  on public.character_dice for update
  using (exists (select 1 from public.characters c
                 where c.id = character_id
                   and (c.owner_uid = auth.uid() or public.is_game_dm(c.game_id))))
  with check (exists (select 1 from public.characters c
                      where c.id = character_id
                        and (c.owner_uid = auth.uid() or public.is_game_dm(c.game_id))));
create policy "character_dice: owner or dm deletes"
  on public.character_dice for delete
  using (exists (select 1 from public.characters c
                 where c.id = character_id
                   and (c.owner_uid = auth.uid() or public.is_game_dm(c.game_id))));


-- =====================================================================
-- SKILL PROMPTS  (seeds for the AI narrative generator)
-- =====================================================================

create table public.skill_prompts (
  id         uuid primary key default gen_random_uuid(),
  key        text not null,
  game_id    uuid references public.games(id) on delete cascade,
  label      text not null,
  facts      text[] not null default '{}',
  gear_tags  text[] not null default '{}',
  guidance   text
);

comment on table public.skill_prompts is
  'Per-roll-kind hints for generated narrative: which character facts to mention and which carried gear is relevant. From SKILL_PROMPT_SEEDS in characternarrative.js. 34 keys: 18 skills, 6 saves, 6 checks, custom, attack, spell, death.';
comment on column public.skill_prompts.key is
  'The roll kind. Skill keys as in skills.key; str_save .. cha_save; str_check .. cha_check; custom, attack, spell, death.';
comment on column public.skill_prompts.game_id is
  'NULL means global. Set means this campaign only.';
comment on column public.skill_prompts.facts is
  'Dossier sections to include in the prompt. Vocabulary: ability, armor, wounds, faith, look, weapons, size, gear, senses. Was a comma-joined string.';
comment on column public.skill_prompts.gear_tags is
  'Substrings that mark an inventory item as relevant to this roll. Was a pipe-joined string.';
comment on column public.skill_prompts.guidance is
  'One sentence of tone direction for the generator.';

create unique index skill_prompts_global_key_idx
  on public.skill_prompts(key) where game_id is null;
create unique index skill_prompts_game_key_idx
  on public.skill_prompts(key, game_id) where game_id is not null;

alter table public.skill_prompts enable row level security;

create policy "skill_prompts: read global or own game"
  on public.skill_prompts for select
  using (game_id is null or public.is_game_member(game_id));
create policy "skill_prompts: dm writes own game"
  on public.skill_prompts for insert
  with check (game_id is not null and public.is_game_dm(game_id));
create policy "skill_prompts: dm updates own game"
  on public.skill_prompts for update
  using (game_id is not null and public.is_game_dm(game_id))
  with check (game_id is not null and public.is_game_dm(game_id));
create policy "skill_prompts: dm deletes own game"
  on public.skill_prompts for delete
  using (game_id is not null and public.is_game_dm(game_id));


-- =====================================================================
-- SPELLS  (ported as-is — see the header)
-- =====================================================================

create table public.spells (
  id             uuid primary key default gen_random_uuid(),
  key            text not null,
  game_id        uuid references public.games(id) on delete cascade,
  name           text not null,
  roll_name      text not null,
  level          integer not null check (level between 0 and 9),
  cast_type      text not null,
  category       text,
  school         text,
  save_ability   public.ability_code,
  dice           text,
  spell_atk      text,
  dc             integer,
  concentration  boolean not null default false,
  ritual         boolean not null default false,
  range          text,
  duration       text,
  special_text   text,
  guidance       text,
  description    text
);

comment on table public.spells is
  'Spell catalogue. Ported from the Spellbook tab with one character''s numbers baked in — dice, spell_atk and dc are literals here until the spell module derives them. Prepared was per-character state and was not ported.';
comment on column public.spells.key is
  'sp_ prefixed slug, e.g. sp_sacredflame. Upcast variants were separate rows (sp_curewounds2) and stay that way for now.';
comment on column public.spells.game_id is
  'NULL means global. Set means this campaign only.';
comment on column public.spells.roll_name is
  'What a player types to cast it. Lowercase, matches resolveRequest vocabulary.';
comment on column public.spells.level is
  '0 is a cantrip.';
comment on column public.spells.cast_type is
  'Save | Attack | Heal | Utility. Decides which roll, if any, casting produces.';
comment on column public.spells.category is
  'Display grouping from the source: Cantrips, 1st Circle, 2nd Circle ... Not a rule.';
comment on column public.spells.save_ability is
  'For cast_type Save: the ability the target saves with.';
comment on column public.spells.dice is
  'BAKED. Damage or healing formula as shipped, with the caster''s modifier already added where the source had it (2d8+4). The spell module should replace this with a base formula plus a derived modifier.';
comment on column public.spells.spell_atk is
  'BAKED. The spell attack bonus as shipped (+7). Should be derived: proficiency + casting ability modifier.';
comment on column public.spells.dc is
  'BAKED. Save DC as shipped (15). Should be derived: 8 + proficiency + casting ability modifier.';
comment on column public.spells.special_text is
  'Rules text shown on the card.';
comment on column public.spells.guidance is
  'Tone direction for the narrative generator.';

create unique index spells_global_key_idx
  on public.spells(key) where game_id is null;
create unique index spells_game_key_idx
  on public.spells(key, game_id) where game_id is not null;

alter table public.spells enable row level security;

create policy "spells: read global or own game"
  on public.spells for select
  using (game_id is null or public.is_game_member(game_id));
create policy "spells: dm writes own game"
  on public.spells for insert
  with check (game_id is not null and public.is_game_dm(game_id));
create policy "spells: dm updates own game"
  on public.spells for update
  using (game_id is not null and public.is_game_dm(game_id))
  with check (game_id is not null and public.is_game_dm(game_id));
create policy "spells: dm deletes own game"
  on public.spells for delete
  using (game_id is not null and public.is_game_dm(game_id));


-- =====================================================================
-- TECHNIQUES  (ported as-is — see the header)
-- =====================================================================

create table public.techniques (
  id            uuid primary key default gen_random_uuid(),
  key           text not null,
  game_id       uuid references public.games(id) on delete cascade,
  name          text not null,
  roll_name     text not null,
  weapon        text not null,
  category      text,
  tier          text,
  min_level     integer not null default 1 check (min_level between 1 and 20),
  dice          text not null,
  crit_min      integer not null default 20 check (crit_min between 2 and 20),
  fumble_max    integer not null default 1 check (fumble_max between 0 and 19),
  special_text  text,
  guidance      text
);

comment on table public.techniques is
  'Weapon techniques: a named attack with its own damage dice and its own crit and fumble thresholds. Ported from the Techniques tab, written for one character''s weapons.';
comment on column public.techniques.key is
  'Slug. Prefixed by weapon family where the source did: hc_ heavy crossbow, hm_ light hammer melee, ht_ light hammer thrown; the mace techniques are unprefixed.';
comment on column public.techniques.game_id is
  'NULL means global. Set means this campaign only.';
comment on column public.techniques.roll_name is
  'What a player types. Lowercase, punctuation stripped (stones judgment).';
comment on column public.techniques.weapon is
  'Free text naming the weapon this technique belongs to. Matching against inventory is by name for now.';
comment on column public.techniques.tier is
  'Class 1 | Class 2 in the source. Display grouping, not a rule.';
comment on column public.techniques.min_level is
  'Character level at which the technique becomes available.';
comment on column public.techniques.dice is
  'Damage formula. Includes house dice the engine must accept: 1d7, 1d14.';
comment on column public.techniques.crit_min is
  'A natural d20 at or above this is a crit. 20 is standard; 18 and 19 widen the range.';
comment on column public.techniques.fumble_max is
  'A natural d20 at or below this is a fumble. 1 is standard; 0 means the technique cannot fumble; 2 and 3 widen the range.';
comment on column public.techniques.special_text is
  'Rider rules text, shown on the card when present. Some carry emoji from the Discord-embed era; harmless.';
comment on column public.techniques.guidance is
  'Tone direction for the narrative generator.';

create unique index techniques_global_key_idx
  on public.techniques(key) where game_id is null;
create unique index techniques_game_key_idx
  on public.techniques(key, game_id) where game_id is not null;

alter table public.techniques enable row level security;

create policy "techniques: read global or own game"
  on public.techniques for select
  using (game_id is null or public.is_game_member(game_id));
create policy "techniques: dm writes own game"
  on public.techniques for insert
  with check (game_id is not null and public.is_game_dm(game_id));
create policy "techniques: dm updates own game"
  on public.techniques for update
  using (game_id is not null and public.is_game_dm(game_id))
  with check (game_id is not null and public.is_game_dm(game_id));
create policy "techniques: dm deletes own game"
  on public.techniques for delete
  using (game_id is not null and public.is_game_dm(game_id));


-- =====================================================================
-- SEED: global content
-- =====================================================================
-- Generated from Application Data.xlsx (tabs DiceSets, Dice, Narratives,
-- SkillPrompts, Spellbook, Techniques). Idempotent — every conflict
-- target is the global partial index. Row counts: 4, 80, 360, 34, 24,
-- 31 = 533.

insert into public.dice_sets (key, game_id, name, price, preview_url) values
  ('001', null, 'Blue Crystal', 0, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_20b.png'),
  ('002', null, 'Gold', 0, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_g20.png'),
  ('003', null, 'Midnight', 0, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_m20.png'),
  ('004', null, 'Red Crystal', 0, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_r20.png')
on conflict (key) where game_id is null do nothing;

insert into public.dice_faces (set_key, game_id, die, face, image_url) values
  ('001', null, 'd20', 1, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_1b.png'),
  ('001', null, 'd20', 2, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_2b.png'),
  ('001', null, 'd20', 3, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_3b.png'),
  ('001', null, 'd20', 4, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_4b.png'),
  ('001', null, 'd20', 5, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_5b.png'),
  ('001', null, 'd20', 6, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_6b.png'),
  ('001', null, 'd20', 7, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_7b.png'),
  ('001', null, 'd20', 8, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_8b.png'),
  ('001', null, 'd20', 9, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_9b.png'),
  ('001', null, 'd20', 10, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_10b.png'),
  ('001', null, 'd20', 11, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_11b.png'),
  ('001', null, 'd20', 12, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_12b.png'),
  ('001', null, 'd20', 13, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_13b.png'),
  ('001', null, 'd20', 14, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_14b.png'),
  ('001', null, 'd20', 15, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_15b.png'),
  ('001', null, 'd20', 16, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_16b.png'),
  ('001', null, 'd20', 17, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_17b.png'),
  ('001', null, 'd20', 18, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_18b.png'),
  ('001', null, 'd20', 19, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_19b.png'),
  ('001', null, 'd20', 20, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_20b.png'),
  ('002', null, 'd20', 1, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_g1.png'),
  ('002', null, 'd20', 2, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_g2.png'),
  ('002', null, 'd20', 3, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_g3.png'),
  ('002', null, 'd20', 4, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_g4.png'),
  ('002', null, 'd20', 5, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_g5.png'),
  ('002', null, 'd20', 6, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_g6.png'),
  ('002', null, 'd20', 7, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_g7.png'),
  ('002', null, 'd20', 8, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_g8.png'),
  ('002', null, 'd20', 9, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_g9.png'),
  ('002', null, 'd20', 10, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_g10.png'),
  ('002', null, 'd20', 11, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_g11.png'),
  ('002', null, 'd20', 12, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_g12.png'),
  ('002', null, 'd20', 13, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_g13.png'),
  ('002', null, 'd20', 14, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_g14.png'),
  ('002', null, 'd20', 15, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_g15.png'),
  ('002', null, 'd20', 16, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_g16.png'),
  ('002', null, 'd20', 17, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_g17.png'),
  ('002', null, 'd20', 18, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_g18.png'),
  ('002', null, 'd20', 19, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_g19.png'),
  ('002', null, 'd20', 20, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_g20.png'),
  ('003', null, 'd20', 1, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_m1.png'),
  ('003', null, 'd20', 2, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_m2.png'),
  ('003', null, 'd20', 3, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_m3.png'),
  ('003', null, 'd20', 4, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_m4.png'),
  ('003', null, 'd20', 5, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_m5.png'),
  ('003', null, 'd20', 6, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_m6.png'),
  ('003', null, 'd20', 7, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_m7.png'),
  ('003', null, 'd20', 8, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_m8.png'),
  ('003', null, 'd20', 9, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_m9.png'),
  ('003', null, 'd20', 10, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_m10.png'),
  ('003', null, 'd20', 11, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_m11.png'),
  ('003', null, 'd20', 12, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_m12.png'),
  ('003', null, 'd20', 13, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_m13.png'),
  ('003', null, 'd20', 14, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_m14.png'),
  ('003', null, 'd20', 15, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_m15.png'),
  ('003', null, 'd20', 16, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_m16.png'),
  ('003', null, 'd20', 17, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_m17.png'),
  ('003', null, 'd20', 18, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_m18.png'),
  ('003', null, 'd20', 19, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_m19.png'),
  ('003', null, 'd20', 20, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_m20.png'),
  ('004', null, 'd20', 1, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_r1.png'),
  ('004', null, 'd20', 2, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_r2.png'),
  ('004', null, 'd20', 3, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_r3.png'),
  ('004', null, 'd20', 4, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_r4.png'),
  ('004', null, 'd20', 5, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_r5.png'),
  ('004', null, 'd20', 6, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_r6.png'),
  ('004', null, 'd20', 7, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_r7.png'),
  ('004', null, 'd20', 8, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_r8.png'),
  ('004', null, 'd20', 9, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_r9.png'),
  ('004', null, 'd20', 10, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_r10.png'),
  ('004', null, 'd20', 11, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_r11.png'),
  ('004', null, 'd20', 12, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_r12.png'),
  ('004', null, 'd20', 13, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_r13.png'),
  ('004', null, 'd20', 14, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_r14.png'),
  ('004', null, 'd20', 15, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_r15.png'),
  ('004', null, 'd20', 16, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_r16.png'),
  ('004', null, 'd20', 17, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_r17.png'),
  ('004', null, 'd20', 18, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_r18.png'),
  ('004', null, 'd20', 19, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_r19.png'),
  ('004', null, 'd20', 20, 'https://assets.forge-vtt.com/682698e464a8e01c529f02a4/Dice/d20_r20.png')
on conflict (set_key, die, face) where game_id is null do nothing;

insert into public.narrative_lines (pack, key, seq, game_id, line, enabled) values
  ('base', 'acr', 1, null, 'They shift their weight onto the balls of their feet, arms drifting wide, letting the body hunt its own line through the gap ahead.', true),
  ('base', 'acr', 2, null, 'A breath, a crouch, and they commit — hips leading, shoulders trailing, armor dragging half a beat behind the motion.', true),
  ('base', 'acr', 3, null, 'One hand plants and the legs swing through, trusting momentum to carry what strength alone would not.', true),
  ('base', 'acr', 4, null, 'The narrow footing gets tested with a toe, then with everything, the whole body strung tight above the drop.', true),
  ('base', 'acr', 5, null, 'They tuck into the fall rather than fight it, rolling to meet the ground on terms of their own choosing.', true),
  ('base', 'acr', 6, null, 'Weight rides forward, then back, the balance point hunted by inches while the gear shifts against every correction.', true),
  ('base', 'acr', 7, null, 'Mid-stride they twist, threading a shoulder through first, the rest of them unspooling after it.', true),
  ('base', 'acr', 8, null, 'Knees bend, breath drops low, and they spring — the takeoff clean, everything after it still open.', true),
  ('base', 'acr', 9, null, 'They flatten to the surface and slide, palms braking, boots hunting for anything that will hold.', true),
  ('base', 'acr', 10, null, 'An arm sweeps wide for counterweight as they step onto the edge, negotiating hard with everything they carry.', true),
  ('base', 'ani', 1, null, 'They stop moving entirely, letting the animal close the distance, hands loose and open at their sides.', true),
  ('base', 'ani', 2, null, 'The voice drops low and even, the same handful of syllables repeated, meaning nothing and promising everything.', true),
  ('base', 'ani', 3, null, 'They crouch to put their eyes below the animal''s, palms turned up, waiting out the flattened ears.', true),
  ('base', 'ani', 4, null, 'A slow half-step, then stillness. Another. The approach measured against the twitch of a tail.', true),
  ('base', 'ani', 5, null, 'They breathe out long and slow, willing the tension out of their shoulders before it travels down the lead.', true),
  ('base', 'ani', 6, null, 'An offering is held at arm''s length and simply left there, the hand steady, the gaze somewhere else.', true),
  ('base', 'ani', 7, null, 'They turn slightly aside, presenting a shoulder instead of a chest, and let the silence do the work.', true),
  ('base', 'ani', 8, null, 'Fingers find the place along the neck where the muscle is bunched, and rest there without pressing.', true),
  ('base', 'ani', 9, null, 'They match the animal''s rhythm — the shifting, the snorting — and wait for it to slow to theirs.', true),
  ('base', 'ani', 10, null, 'The hand comes up open and stops short, hovering, letting the animal decide the last few inches.', true),
  ('base', 'arc', 1, null, 'They trace the shape in the air a hand''s breadth from the surface, matching it against something half-remembered.', true),
  ('base', 'arc', 2, null, 'The eyes narrow and travel the pattern from its outermost ring inward, hunting the break in the sequence.', true),
  ('base', 'arc', 3, null, 'They mouth the syllables without sound, testing which reading the marks will tolerate.', true),
  ('base', 'arc', 4, null, 'A finger hovers over one glyph while the mind runs back through everything ever read about its kin.', true),
  ('base', 'arc', 5, null, 'They tilt their head, reading the working sideways, the way a copyist learns to catch a forger.', true),
  ('base', 'arc', 6, null, 'The remembered page rises behind their eyes and they hold it there, comparing it line for line with what stands in front of them.', true),
  ('base', 'arc', 7, null, 'They count the repetitions under their breath, looking for the one that does not belong.', true),
  ('base', 'arc', 8, null, 'Fingertips stop short of contact, mapping the flow of the thing by the way the air behaves above it.', true),
  ('base', 'arc', 9, null, 'They sort the marks into the ones they know, the ones they half-know, and the one that answers to nothing.', true),
  ('base', 'arc', 10, null, 'The whole structure gets held in the mind at once, turned, and set down again from a different side.', true),
  ('base', 'ath', 1, null, 'They set their grip, drag in a breath, and pull — shoulders bunching, the whole weight of their gear hanging off the effort.', true),
  ('base', 'ath', 2, null, 'Boots dig in and the legs drive, the body angled hard against whatever is refusing to move.', true),
  ('base', 'ath', 3, null, 'Both hands lock on and they haul, arms trembling, everything they are carrying trying to take them back down.', true),
  ('base', 'ath', 4, null, 'They plant a shoulder and shove, driving from the hips, breath forced out through clenched teeth.', true),
  ('base', 'ath', 5, null, 'One arm reaches for the next hold while the other takes all of it, the fingers whitening.', true),
  ('base', 'ath', 6, null, 'They gather themselves low and explode upward, the gear slapping against them as they rise.', true),
  ('base', 'ath', 7, null, 'The muscles across their back draw taut and hold, taking the strain in one long unbroken pull.', true),
  ('base', 'ath', 8, null, 'They wedge themselves in, brace against both sides, and start working upward by inches.', true),
  ('base', 'ath', 9, null, 'A run-up, three strides, and they throw everything forward — arms wheeling, the ground already gone.', true),
  ('base', 'ath', 10, null, 'They set their feet, take the load, and lift, the strain climbing up out of the legs into the jaw.', true),
  ('base', 'dec', 1, null, 'The face settles into something easy and unbothered, and the lie comes out in the same voice as the weather.', true),
  ('base', 'dec', 2, null, 'They let a small, useless truth go first, watching to see how hungrily it gets taken.', true),
  ('base', 'dec', 3, null, 'The hands stay visible and still, doing nothing, saying nothing, which is the whole trick of it.', true),
  ('base', 'dec', 4, null, 'They answer half a beat too quickly, then slow down, dressing the story in the hesitations it needs.', true),
  ('base', 'dec', 5, null, 'A shrug, a half-smile, and the story arrives already worn in, as if it had been true for years.', true),
  ('base', 'dec', 6, null, 'They meet the eyes just long enough, then look away at exactly the moment an honest person would.', true),
  ('base', 'dec', 7, null, 'The voice picks up a little warmth, a little boredom, the tone of a person with nothing to gain.', true),
  ('base', 'dec', 8, null, 'They repeat the question back, buying the breath they need to decide which version to be.', true),
  ('base', 'dec', 9, null, 'Details get offered freely — the wrong ones, the harmless ones, the ones that make a listener stop asking.', true),
  ('base', 'dec', 10, null, 'They lean into the story rather than defend it, and let the confidence carry what the facts cannot.', true),
  ('base', 'his', 1, null, 'They reach back through years of read pages for the name that goes with the mark in front of them.', true),
  ('base', 'his', 2, null, 'The dates line themselves up, one after another, and they walk the chain backward looking for the break.', true),
  ('base', 'his', 3, null, 'They dredge for the old quarrel — who held this ground before, and who took it, and what it cost.', true),
  ('base', 'his', 4, null, 'A half-remembered account surfaces, and they turn it over, testing which parts were ever verified.', true),
  ('base', 'his', 5, null, 'They set what they know against what they are being told and look hard at the gap between them.', true),
  ('base', 'his', 6, null, 'The lineage assembles itself in their head, generation by generation, until it reaches whoever matters.', true),
  ('base', 'his', 7, null, 'They cast back for the treaty, the terms, the clause everyone stopped honoring first.', true),
  ('base', 'his', 8, null, 'Names arrive out of order and they sort them, discarding the ones that belong to a different century.', true),
  ('base', 'his', 9, null, 'The story as it is usually told comes first; then, more slowly, the version that was written down.', true),
  ('base', 'his', 10, null, 'They hunt for the precedent — the last time this happened, and what came of it.', true),
  ('base', 'ins', 1, null, 'They stop listening to the words and start watching the hands, waiting for the two to disagree.', true),
  ('base', 'ins', 2, null, 'The eyes hold steady on the other''s face, tracking the small corrections around the mouth.', true),
  ('base', 'ins', 3, null, 'They let the silence stretch, and watch what gets rushed in to fill it.', true),
  ('base', 'ins', 4, null, 'Attention narrows to the pause before each answer — its length, and whether it belongs there.', true),
  ('base', 'ins', 5, null, 'They weigh what is being said against everything that has not been said yet.', true),
  ('base', 'ins', 6, null, 'The gaze drops to the shoulders, the shift of weight, the tell that arrives before the sentence does.', true),
  ('base', 'ins', 7, null, 'They ask nothing and simply wait, letting the other decide how much of the quiet to spend.', true),
  ('base', 'ins', 8, null, 'Something in the phrasing snags, and they turn it over without letting their face change.', true),
  ('base', 'ins', 9, null, 'They listen past the argument for the thing underneath it — what this person actually wants.', true),
  ('base', 'ins', 10, null, 'The breathing gets watched more closely than the words, hunting the moment it changes.', true),
  ('base', 'itm', 1, null, 'They go completely still, and let the stillness say what raising a voice never could.', true),
  ('base', 'itm', 2, null, 'The weight of the weapon shifts, deliberately, into plain view — and nothing is said at all.', true),
  ('base', 'itm', 3, null, 'They close the distance by one unhurried step and simply stand there, filling more of the room than before.', true),
  ('base', 'itm', 4, null, 'The voice drops rather than rises, quiet enough that everyone leans in to catch it.', true),
  ('base', 'itm', 5, null, 'They hold the eye contact past the point of comfort and let it keep going.', true),
  ('base', 'itm', 6, null, 'A single sentence, flat and unhurried, with the promise of the next one left unspoken.', true),
  ('base', 'itm', 7, null, 'They square up, shoulders turning fully toward the other, taking up every inch they have.', true),
  ('base', 'itm', 8, null, 'The hand comes to rest somewhere useful, without hurry, and stays there.', true),
  ('base', 'itm', 9, null, 'They let the other finish, wait a beat too long, and answer with the smallest possible movement.', true),
  ('base', 'itm', 10, null, 'Nothing in the posture threatens. Everything in it promises.', true),
  ('base', 'inv', 1, null, 'They start at the edges and work inward, hands behind their back, taking nothing on trust.', true),
  ('base', 'inv', 2, null, 'The seam gets followed the whole way around, fingers stopping at every place the join is wrong.', true),
  ('base', 'inv', 3, null, 'They crouch to bring their eye level with the surface, hunting the detail that does not belong.', true),
  ('base', 'inv', 4, null, 'Everything gets moved once, deliberately, and set back exactly where it was.', true),
  ('base', 'inv', 5, null, 'They count what is there against what should be there, and the difference is where they linger.', true),
  ('base', 'inv', 6, null, 'A palm passes over the surface, reading it by touch where the light is not enough.', true),
  ('base', 'inv', 7, null, 'They work the corners first — the places nobody bothers to clean, and nobody bothers to fake.', true),
  ('base', 'inv', 8, null, 'The dust gets read like a page: what has been disturbed, what has not, and how recently.', true),
  ('base', 'inv', 9, null, 'They step back to take in the whole of it, then step in again on the one thing that snagged.', true),
  ('base', 'inv', 10, null, 'Each possibility gets tested and set aside, narrowing by elimination toward what is left.', true),
  ('base', 'med', 1, null, 'The hands go to the wound directly, unhurried, pressing where pressure is needed and nowhere else.', true),
  ('base', 'med', 2, null, 'They strip back what covers the injury and look first, before touching anything at all.', true),
  ('base', 'med', 3, null, 'Fingers walk the ribs one at a time, reading the damage through the skin.', true),
  ('base', 'med', 4, null, 'They tilt the head, clear the airway, and put an ear close to listen for what the chest is doing.', true),
  ('base', 'med', 5, null, 'The bleeding gets found before it gets treated — a hand tracing back along the wet to its source.', true),
  ('base', 'med', 6, null, 'They talk while they work, low and steady, more to slow the breathing than to say anything.', true),
  ('base', 'med', 7, null, 'The limb gets braced against their own body and eased straight, slowly, without asking permission first.', true),
  ('base', 'med', 8, null, 'They read the eyes, then the fingertips, then the pulse, building the picture in order.', true),
  ('base', 'med', 9, null, 'Cloth is folded, packed, and pressed down hard, the weight going through a straight arm.', true),
  ('base', 'med', 10, null, 'They work by touch as much as sight, the hands doing what they have done a hundred times.', true),
  ('base', 'nat', 1, null, 'They read the ground first — what grows here, what does not, and what has been walked through.', true),
  ('base', 'nat', 2, null, 'The sky gets a long look, the wind gets tested on the back of a wet hand.', true),
  ('base', 'nat', 3, null, 'They crouch over the print and measure it against their own spread fingers.', true),
  ('base', 'nat', 4, null, 'The bent stems get followed to where they stop being bent.', true),
  ('base', 'nat', 5, null, 'They sort the sounds of the place into the ones that belong and the one that arrived recently.', true),
  ('base', 'nat', 6, null, 'A leaf gets crushed between finger and thumb and brought up to the nose.', true),
  ('base', 'nat', 7, null, 'They watch which way the small things have fled, and reason backward from there.', true),
  ('base', 'nat', 8, null, 'The water gets studied — its clarity, its edge, what has been drinking from it.', true),
  ('base', 'nat', 9, null, 'They put a hand flat on the trunk and read the season out of the bark.', true),
  ('base', 'nat', 10, null, 'The whole clearing gets taken in slowly, the way a page is read rather than glanced at.', true),
  ('base', 'prc', 1, null, 'They stop, breathe out, and let the attention go wide — no single thing looked at, everything watched.', true),
  ('base', 'prc', 2, null, 'The head turns by degrees, sweeping the dark in overlapping arcs, refusing to hurry.', true),
  ('base', 'prc', 3, null, 'They hold their breath so the sound of it stops competing with everything else.', true),
  ('base', 'prc', 4, null, 'The eyes go first to the places a thing would stand if it did not want to be seen.', true),
  ('base', 'prc', 5, null, 'They stand very still and let the scene resolve, waiting for whatever moves first.', true),
  ('base', 'prc', 6, null, 'Attention drops from the horizon to the ground and back, hunting the seam between them.', true),
  ('base', 'prc', 7, null, 'They tilt their head to bring one ear forward, hunting the sound underneath the obvious one.', true),
  ('base', 'prc', 8, null, 'The scan starts wide and closes inward, tightening on the one place the light behaves oddly.', true),
  ('base', 'prc', 9, null, 'They let their eyes go slightly unfocused, the way you do to catch movement at the edge.', true),
  ('base', 'prc', 10, null, 'Every sense gets cast out at once, spread thin across the whole of it, waiting to be snagged.', true),
  ('base', 'prf', 1, null, 'They fill their chest, find the back wall with their eyes, and let the first note go.', true),
  ('base', 'prf', 2, null, 'The room gets a moment to settle before they begin, and the waiting is part of it.', true),
  ('base', 'prf', 3, null, 'They pitch the voice to carry past the nearest faces to the ones half-turned away.', true),
  ('base', 'prf', 4, null, 'Hands and voice arrive together, and the space stops belonging to the audience.', true),
  ('base', 'prf', 5, null, 'They start smaller than the room, drawing it in, making it come the rest of the way.', true),
  ('base', 'prf', 6, null, 'The opening is delivered straight at the one face that has not looked up yet.', true),
  ('base', 'prf', 7, null, 'They let the silence run a beat past comfortable, and step into it.', true),
  ('base', 'prf', 8, null, 'The rhythm sets first, in the feet, before anything else is offered.', true),
  ('base', 'prf', 9, null, 'They give the crowd the shape of what is coming, then take their time getting there.', true),
  ('base', 'prf', 10, null, 'Everything narrows to the space between the first breath and whatever the room decides.', true),
  ('base', 'per', 1, null, 'They lead with the thing the other already believes, and build from that stone outward.', true),
  ('base', 'per', 2, null, 'The voice stays warm and unhurried, making room rather than pressing into it.', true),
  ('base', 'per', 3, null, 'They put the objection into words first, better than the other could, and then answer it.', true),
  ('base', 'per', 4, null, 'The argument gets framed as something already half-agreed, needing only a nod to finish.', true),
  ('base', 'per', 5, null, 'They ask a question they know the answer to, and let the other arrive there alone.', true),
  ('base', 'per', 6, null, 'Hands open, weight back, nothing about them crowding the space between.', true),
  ('base', 'per', 7, null, 'They concede the small point immediately and completely, and watch it buy them the larger one.', true),
  ('base', 'per', 8, null, 'The case gets made once, plainly, and then they stop talking and let it sit.', true),
  ('base', 'per', 9, null, 'They speak to what this person stands to lose, not to what they themselves want.', true),
  ('base', 'per', 10, null, 'The tone drops into something confiding, as though the two of them were already on the same side.', true),
  ('base', 'rel', 1, null, 'The symbol comes into the hand and is gripped hard, the way a rail is gripped on a stair.', true),
  ('base', 'rel', 2, null, 'They begin the rite from its opening words, letting the shape of it steady them.', true),
  ('base', 'rel', 3, null, 'The prayer starts silent and only becomes sound halfway through.', true),
  ('base', 'rel', 4, null, 'They kneel without hurry, setting the whole of their attention on what is owed here.', true),
  ('base', 'rel', 5, null, 'The old words come out in the order they were learned, worn smooth from use.', true),
  ('base', 'rel', 6, null, 'They hold the holy thing out in front of them, arm straight, and do not waver.', true),
  ('base', 'rel', 7, null, 'The doctrine arranges itself in their mind — what is permitted, what is required, what is refused.', true),
  ('base', 'rel', 8, null, 'They breathe the invocation out slowly, giving each name its full weight.', true),
  ('base', 'rel', 9, null, 'A hand comes up in the gesture their teachers made, before they knew what it meant.', true),
  ('base', 'rel', 10, null, 'They set their faith in front of the thing like a shield and stand behind it.', true),
  ('base', 'slt', 1, null, 'One hand rises into plain view and does something interesting, and the other simply works.', true),
  ('base', 'slt', 2, null, 'The fingers move once, briefly, in the moment the other''s eyes are elsewhere.', true),
  ('base', 'slt', 3, null, 'They keep the conversation going with their face while their hand finishes.', true),
  ('base', 'slt', 4, null, 'A shoulder turns to screen the movement, casual, exactly as it would turn anyway.', true),
  ('base', 'slt', 5, null, 'The object is gone before the hand that took it has finished passing.', true),
  ('base', 'slt', 6, null, 'They let the noise and the crowd do half the work and supply the rest with two fingers.', true),
  ('base', 'slt', 7, null, 'The palm closes around it in the same motion that reaches past it.', true),
  ('base', 'slt', 8, null, 'They set up the misdirection first and only then commit, unhurried, to the actual theft.', true),
  ('base', 'slt', 9, null, 'Fingers walk the seam of the pocket, patient, taking the time not to snag.', true),
  ('base', 'slt', 10, null, 'The whole thing happens inside one gesture that would have looked ordinary from any angle.', true),
  ('base', 'ste', 1, null, 'The feet come down heel-last, weight tested before it is given, every step a decision.', true),
  ('base', 'ste', 2, null, 'They breathe shallow through the mouth and press themselves into the deepest part of the shadow.', true),
  ('base', 'ste', 3, null, 'Everything metal gets stilled with a hand before the body moves at all.', true),
  ('base', 'ste', 4, null, 'They wait out the noise of the room and travel only inside it.', true),
  ('base', 'ste', 5, null, 'The route gets chosen for its footing, not its distance, and taken slowly.', true),
  ('base', 'ste', 6, null, 'They flatten to the wall and slide along it, keeping their outline inside its shape.', true),
  ('base', 'ste', 7, null, 'A pause at the threshold — listening, counting, and only then crossing.', true),
  ('base', 'ste', 8, null, 'They move when the light moves, and stop when it stops.', true),
  ('base', 'ste', 9, null, 'The gear gets gathered in tight against the body so nothing swings or speaks.', true),
  ('base', 'ste', 10, null, 'They put each foot down as if the floor were owed an apology, and go on.', true),
  ('base', 'sur', 1, null, 'They read the trail for the freshest disturbance and follow it with their eyes before their feet.', true),
  ('base', 'sur', 2, null, 'The wind, the light, and the ground get weighed together into a single guess about the hours ahead.', true),
  ('base', 'sur', 3, null, 'They cast in a slow arc, hunting the place the tracks pick up again.', true),
  ('base', 'sur', 4, null, 'A hand goes into the ash to feel whether it still holds any warmth.', true),
  ('base', 'sur', 5, null, 'They study the treeline for the shape of the land behind it.', true),
  ('base', 'sur', 6, null, 'The kindling gets built small and tight, sheltered from the wind with their own back.', true),
  ('base', 'sur', 7, null, 'They pick the campsite by what is above it and behind it, not by how flat it is.', true),
  ('base', 'sur', 8, null, 'The water gets tasted upstream before anything else is decided.', true),
  ('base', 'sur', 9, null, 'They set a snare where the run narrows, working by the logic of the animal rather than their own.', true),
  ('base', 'sur', 10, null, 'The horizon gets read for weather, and the read is made out loud, to no one in particular.', true),
  ('Rodnar Shieldcrest', 'acr', 1, null, 'Rodnar drops his weight low and lets the scale mail settle before he moves, bargaining with every pound of it.', true),
  ('Rodnar Shieldcrest', 'acr', 2, null, 'He swings a boot out for counterweight, the braided beard swinging after, the whole broad frame committing at once.', true),
  ('Rodnar Shieldcrest', 'acr', 3, null, 'One thick hand catches the edge and he hauls his legs through, momentum doing what a lighter man''s grace would.', true),
  ('Rodnar Shieldcrest', 'acr', 4, null, 'He tests the footing with a toe, feels the stone answer through the sole, and shifts his weight onto it.', true),
  ('Rodnar Shieldcrest', 'acr', 5, null, 'Rodnar tucks a shoulder and turns the fall into a tumble rather than let it choose for him.', true),
  ('Rodnar Shieldcrest', 'acr', 6, null, 'The mace gets clamped tight against his ribs so it cannot swing him off his line.', true),
  ('Rodnar Shieldcrest', 'acr', 7, null, 'He turns sideways to thread the gap, pauldrons scraping, breath held against the squeeze.', true),
  ('Rodnar Shieldcrest', 'acr', 8, null, 'Knees bend deep, the armor creaks, and Rodnar launches — heavier than he looks and faster than he should be.', true),
  ('Rodnar Shieldcrest', 'acr', 9, null, 'He flattens against the slope and slides, palms dragging, boots hunting for a lip of rock.', true),
  ('Rodnar Shieldcrest', 'acr', 10, null, 'Both arms come out wide as he steps onto the narrow place, the weight of him arguing with every inch.', true),
  ('Rodnar Shieldcrest', 'ani', 1, null, 'Rodnar stops dead and lets his hands hang open, a broad man doing his best impression of a boulder.', true),
  ('Rodnar Shieldcrest', 'ani', 2, null, 'The voice comes out low and gravelled, saying nothing in particular, the same few syllables again and again.', true),
  ('Rodnar Shieldcrest', 'ani', 3, null, 'He crouches until his teal eyes sit below the animal''s, and waits out the flattened ears.', true),
  ('Rodnar Shieldcrest', 'ani', 4, null, 'A ration comes out of the priest''s pack and is set on the ground between them, and Rodnar looks elsewhere.', true),
  ('Rodnar Shieldcrest', 'ani', 5, null, 'He breathes out slowly through the beard, willing the tension out of his shoulders before it reaches the animal.', true),
  ('Rodnar Shieldcrest', 'ani', 6, null, 'Rodnar turns a pauldron toward it rather than his chest, offering bulk instead of threat.', true),
  ('Rodnar Shieldcrest', 'ani', 7, null, 'One calloused hand comes up open and stops short, hovering, letting the animal close the last of it.', true),
  ('Rodnar Shieldcrest', 'ani', 8, null, 'He murmurs something to Shayl under his breath, more habit than prayer, and keeps perfectly still.', true),
  ('Rodnar Shieldcrest', 'ani', 9, null, 'The stone under them carries the animal''s shifting up into his boots, and he matches his own rhythm to it.', true),
  ('Rodnar Shieldcrest', 'ani', 10, null, 'Fingers find the bunched muscle along the neck and rest there without pressing.', true),
  ('Rodnar Shieldcrest', 'arc', 1, null, 'Rodnar traces the sigil a hand''s breadth off the surface, comparing it against everything a temple education left him.', true),
  ('Rodnar Shieldcrest', 'arc', 2, null, 'He narrows his eyes and walks the pattern from its outer ring inward, hunting the place the sequence breaks.', true),
  ('Rodnar Shieldcrest', 'arc', 3, null, 'The syllables get mouthed silently into the beard, tested for which reading the marks will bear.', true),
  ('Rodnar Shieldcrest', 'arc', 4, null, 'He holds the remembered page behind his eyes and sets it against what stands in front of him.', true),
  ('Rodnar Shieldcrest', 'arc', 5, null, 'Rodnar tilts his head and reads the working from the side, the way a mason checks a wall for true.', true),
  ('Rodnar Shieldcrest', 'arc', 6, null, 'A thick finger hovers over one mark while he runs back through every text that ever carried its kin.', true),
  ('Rodnar Shieldcrest', 'arc', 7, null, 'He sorts the marks into what he knows, what he half-knows, and the one that answers to nothing.', true),
  ('Rodnar Shieldcrest', 'arc', 8, null, 'The lamp gets lifted closer, and he leans in until the light picks out the incised depth of it.', true),
  ('Rodnar Shieldcrest', 'arc', 9, null, 'He counts the repetitions under his breath, waiting for the one that does not belong.', true),
  ('Rodnar Shieldcrest', 'arc', 10, null, 'Rodnar takes the whole structure into his head at once, turns it, and sets it down from another side.', true),
  ('Rodnar Shieldcrest', 'ath', 1, null, 'He sets his grip, fills that broad chest, and pulls — the scale mail dragging at every inch of it.', true),
  ('Rodnar Shieldcrest', 'ath', 2, null, 'Boots dig into the stone and the Unt''gar legs drive, all of that weathered bulk angled into the work.', true),
  ('Rodnar Shieldcrest', 'ath', 3, null, 'Rodnar plants a shoulder and shoves from the hips, breath forced out through clenched teeth.', true),
  ('Rodnar Shieldcrest', 'ath', 4, null, 'Both hands lock on and he hauls, the arms trembling, the mace and the pack fighting him the whole way.', true),
  ('Rodnar Shieldcrest', 'ath', 5, null, 'One hand reaches for the next hold while the other takes all of him, and all of him is considerable.', true),
  ('Rodnar Shieldcrest', 'ath', 6, null, 'He gathers low and drives upward, armor slapping against him as he rises.', true),
  ('Rodnar Shieldcrest', 'ath', 7, null, 'The muscles bunch across his back and hold, taking the strain in one long unbroken pull.', true),
  ('Rodnar Shieldcrest', 'ath', 8, null, 'Rodnar wedges himself between the two faces, braces, and starts working up by inches.', true),
  ('Rodnar Shieldcrest', 'ath', 9, null, 'Three strides and he throws everything forward, the ground already gone from under his boots.', true),
  ('Rodnar Shieldcrest', 'ath', 10, null, 'He sets his feet, takes the load, and lifts — the strain climbing out of the legs into the jaw.', true),
  ('Rodnar Shieldcrest', 'dec', 1, null, 'The weathered face settles into something patient and unremarkable, and the untruth arrives in the same voice as a blessing.', true),
  ('Rodnar Shieldcrest', 'dec', 2, null, 'Rodnar offers a small, harmless truth first, and watches how hungrily it gets taken.', true),
  ('Rodnar Shieldcrest', 'dec', 3, null, 'His hands stay open and still on the table, doing nothing, which is most of the trick.', true),
  ('Rodnar Shieldcrest', 'dec', 4, null, 'He answers a half-beat quick, then slows, dressing the story in the hesitations an honest man would have.', true),
  ('Rodnar Shieldcrest', 'dec', 5, null, 'A shrug moves the pauldrons, and the story comes out already worn in, as if it had been true for years.', true),
  ('Rodnar Shieldcrest', 'dec', 6, null, 'He meets the eyes just long enough, then looks away exactly when a man with nothing to hide would.', true),
  ('Rodnar Shieldcrest', 'dec', 7, null, 'The voice picks up the warmth he uses at gravesides, and puts it behind something untrue.', true),
  ('Rodnar Shieldcrest', 'dec', 8, null, 'Rodnar repeats the question back through the beard, buying the breath to decide who to be.', true),
  ('Rodnar Shieldcrest', 'dec', 9, null, 'He gives away details freely — the wrong ones, the ones that make a listener stop asking.', true),
  ('Rodnar Shieldcrest', 'dec', 10, null, 'He leans into the story rather than defending it, letting the weight of him carry what the facts cannot.', true),
  ('Rodnar Shieldcrest', 'his', 1, null, 'Rodnar reaches back through years of temple reading for the name that belongs to the mark in front of him.', true),
  ('Rodnar Shieldcrest', 'his', 2, null, 'The dates line up in his head and he walks the chain backward, hunting the break.', true),
  ('Rodnar Shieldcrest', 'his', 3, null, 'He dredges for the old grievance — who held this ground before, who took it, and what it cost them.', true),
  ('Rodnar Shieldcrest', 'his', 4, null, 'A half-remembered account surfaces and he turns it over, testing which parts were ever written down.', true),
  ('Rodnar Shieldcrest', 'his', 5, null, 'He sets what he knows against what he is being told and looks hard at the gap.', true),
  ('Rodnar Shieldcrest', 'his', 6, null, 'The lineage assembles itself generation by generation until it arrives at whoever matters here.', true),
  ('Rodnar Shieldcrest', 'his', 7, null, 'Rodnar casts back for the terms of the thing, and for the clause everyone stopped honoring first.', true),
  ('Rodnar Shieldcrest', 'his', 8, null, 'Names come out of order and he sorts them, discarding the ones from the wrong century.', true),
  ('Rodnar Shieldcrest', 'his', 9, null, 'He recalls the version the priests tell, and then, more slowly, the version the records keep.', true),
  ('Rodnar Shieldcrest', 'his', 10, null, 'He hunts for the precedent — the last time this happened, and what it made of the people it happened to.', true),
  ('Rodnar Shieldcrest', 'ins', 1, null, 'Rodnar stops listening to the words and starts watching the hands, waiting for the two to disagree.', true),
  ('Rodnar Shieldcrest', 'ins', 2, null, 'Those teal eyes hold steady, tracking the small corrections at the corners of the other''s mouth.', true),
  ('Rodnar Shieldcrest', 'ins', 3, null, 'He lets the silence run long, and watches what gets rushed in to fill it.', true),
  ('Rodnar Shieldcrest', 'ins', 4, null, 'His attention narrows to the pause before each answer, and whether it belongs there.', true),
  ('Rodnar Shieldcrest', 'ins', 5, null, 'He weighs what is being said against the whole shape of what has not been said yet.', true),
  ('Rodnar Shieldcrest', 'ins', 6, null, 'The gaze drops to the shoulders, to the shift of weight, to the tell that comes before the sentence.', true),
  ('Rodnar Shieldcrest', 'ins', 7, null, 'Rodnar asks nothing at all, and lets the other decide how much of the quiet to spend.', true),
  ('Rodnar Shieldcrest', 'ins', 8, null, 'Something in the phrasing snags, and he turns it over without letting the beard move.', true),
  ('Rodnar Shieldcrest', 'ins', 9, null, 'He listens past the argument for the thing underneath it — what this one actually wants.', true),
  ('Rodnar Shieldcrest', 'ins', 10, null, 'He watches the breathing more closely than the mouth, waiting for the moment it changes.', true),
  ('Rodnar Shieldcrest', 'itm', 1, null, 'Rodnar goes completely still, and lets a broad Unt''gar stillness say what shouting could not.', true),
  ('Rodnar Shieldcrest', 'itm', 2, null, 'The Mace of the Deep Song shifts, deliberately, into plain view — and he says nothing at all.', true),
  ('Rodnar Shieldcrest', 'itm', 3, null, 'He closes the distance by one unhurried step and simply stands there, taking up more of the room.', true),
  ('Rodnar Shieldcrest', 'itm', 4, null, 'The voice drops rather than rises, low enough through the beard that everyone leans in.', true),
  ('Rodnar Shieldcrest', 'itm', 5, null, 'He holds the eye contact past the point of comfort, and keeps holding it.', true),
  ('Rodnar Shieldcrest', 'itm', 6, null, 'One flat sentence, unhurried, with the next one left where everyone can imagine it.', true),
  ('Rodnar Shieldcrest', 'itm', 7, null, 'Rodnar squares up, both slate-blue pauldrons turning fully toward the other.', true),
  ('Rodnar Shieldcrest', 'itm', 8, null, 'His hand comes to rest on the mace haft without hurry, and stays.', true),
  ('Rodnar Shieldcrest', 'itm', 9, null, 'He lets the other finish, waits a beat too long, and answers with the smallest movement he has.', true),
  ('Rodnar Shieldcrest', 'itm', 10, null, 'Nothing in the way he stands is a threat. Everything in it is a promise.', true),
  ('Rodnar Shieldcrest', 'inv', 1, null, 'Rodnar starts at the edges and works inward, thick hands clasped behind his back, taking nothing on trust.', true),
  ('Rodnar Shieldcrest', 'inv', 2, null, 'He follows the seam the whole way around, stopping his finger at every place the join goes wrong.', true),
  ('Rodnar Shieldcrest', 'inv', 3, null, 'The lamp comes down low and he crouches with it, bringing his eye level to the surface.', true),
  ('Rodnar Shieldcrest', 'inv', 4, null, 'Everything gets moved once, deliberately, and set back precisely where it sat.', true),
  ('Rodnar Shieldcrest', 'inv', 5, null, 'He counts what is here against what ought to be here, and lingers on the difference.', true),
  ('Rodnar Shieldcrest', 'inv', 6, null, 'A broad palm passes over the stone, reading by touch what the light will not give him.', true),
  ('Rodnar Shieldcrest', 'inv', 7, null, 'Rodnar works the corners first — the places nobody cleans, and nobody thinks to fake.', true),
  ('Rodnar Shieldcrest', 'inv', 8, null, 'He reads the dust like a page: what has been disturbed, what has not, and how lately.', true),
  ('Rodnar Shieldcrest', 'inv', 9, null, 'He steps back to take in the whole of it, then steps in again on the one thing that snagged.', true),
  ('Rodnar Shieldcrest', 'inv', 10, null, 'Each possibility gets tested and set aside, narrowing by elimination toward whatever is left.', true),
  ('Rodnar Shieldcrest', 'med', 1, null, 'Rodnar''s calloused hands go to the wound directly, pressing where pressure is wanted and nowhere else.', true),
  ('Rodnar Shieldcrest', 'med', 2, null, 'He strips back what covers the injury and looks first, whispering to Shayl before he touches anything.', true),
  ('Rodnar Shieldcrest', 'med', 3, null, 'Thick fingers walk the ribs one at a time, reading the damage through the skin.', true),
  ('Rodnar Shieldcrest', 'med', 4, null, 'He tilts the head, clears the airway, and brings his ear down close to the chest.', true),
  ('Rodnar Shieldcrest', 'med', 5, null, 'The bleeding gets found before it gets treated, a hand tracing back along the wet to its source.', true),
  ('Rodnar Shieldcrest', 'med', 6, null, 'He talks low and steady while he works, more to slow their breathing than to say anything.', true),
  ('Rodnar Shieldcrest', 'med', 7, null, 'The limb is braced against his own broad frame and eased straight, slowly, without asking first.', true),
  ('Rodnar Shieldcrest', 'med', 8, null, 'Rodnar reads the eyes, then the fingertips, then the pulse, building it in order.', true),
  ('Rodnar Shieldcrest', 'med', 9, null, 'Cloth is folded, packed, and pressed down hard, the weight going through a straight arm.', true),
  ('Rodnar Shieldcrest', 'med', 10, null, 'Divine grace and field physic go into the same wound, in the same motion, from the same hands.', true),
  ('Rodnar Shieldcrest', 'nat', 1, null, 'Rodnar reads the ground first — what grows here, what refuses to, and what has been walked through.', true),
  ('Rodnar Shieldcrest', 'nat', 2, null, 'He wets the back of a broad hand and holds it up, then gives the sky a long look.', true),
  ('Rodnar Shieldcrest', 'nat', 3, null, 'He crouches over the print and measures it against his own spread fingers.', true),
  ('Rodnar Shieldcrest', 'nat', 4, null, 'The bent stems get followed to the place they stop being bent.', true),
  ('Rodnar Shieldcrest', 'nat', 5, null, 'Rodnar sorts the sounds of the place into what belongs and what arrived recently.', true),
  ('Rodnar Shieldcrest', 'nat', 6, null, 'A leaf gets crushed between finger and thumb and brought up under the beard to his nose.', true),
  ('Rodnar Shieldcrest', 'nat', 7, null, 'He sets a boot flat on the ground and lets the tremor of the place come up through the sole.', true),
  ('Rodnar Shieldcrest', 'nat', 8, null, 'The water gets studied — its clarity, its margin, what has been drinking from it.', true),
  ('Rodnar Shieldcrest', 'nat', 9, null, 'He puts a hand on the trunk and reads the season out of the bark.', true),
  ('Rodnar Shieldcrest', 'nat', 10, null, 'The whole clearing gets taken in slowly, the way a page is read rather than glanced at.', true),
  ('Rodnar Shieldcrest', 'prc', 1, null, 'Rodnar plants his boots and lets the stone speak into them, sending his attention wide and low.', true),
  ('Rodnar Shieldcrest', 'prc', 2, null, 'He stops, breathes out, and lets his gaze go loose — nothing looked at, everything watched.', true),
  ('Rodnar Shieldcrest', 'prc', 3, null, 'The head turns by degrees, sweeping the dark in overlapping arcs, refusing to be hurried.', true),
  ('Rodnar Shieldcrest', 'prc', 4, null, 'He holds his breath so the sound of it stops competing with the room.', true),
  ('Rodnar Shieldcrest', 'prc', 5, null, 'Rodnar lifts the lamp high and looks past its glare, into the part of the dark it cannot reach.', true),
  ('Rodnar Shieldcrest', 'prc', 6, null, 'Darkvision does the far work while the tremorsense does the near, and he holds both at once.', true),
  ('Rodnar Shieldcrest', 'prc', 7, null, 'His eyes go first to the places a thing would stand if it did not wish to be seen.', true),
  ('Rodnar Shieldcrest', 'prc', 8, null, 'He tilts his head to bring one ear forward, hunting the sound underneath the obvious one.', true),
  ('Rodnar Shieldcrest', 'prc', 9, null, 'The scan starts wide and closes inward, tightening where the shadow behaves wrongly.', true),
  ('Rodnar Shieldcrest', 'prc', 10, null, 'Every sense he has gets cast out at once, spread thin across the whole of it, waiting to be snagged.', true),
  ('Rodnar Shieldcrest', 'prf', 1, null, 'Rodnar fills that broad chest, finds the back wall with his eyes, and lets the first note go.', true),
  ('Rodnar Shieldcrest', 'prf', 2, null, 'He gives the room a moment to settle before he begins, and the waiting is half of it.', true),
  ('Rodnar Shieldcrest', 'prf', 3, null, 'The voice comes out of the beard pitched to carry past the near faces to the ones half-turned away.', true),
  ('Rodnar Shieldcrest', 'prf', 4, null, 'Hands and voice arrive together, and the space stops belonging to the crowd.', true),
  ('Rodnar Shieldcrest', 'prf', 5, null, 'He starts smaller than the room and draws it in, making it come the rest of the way to him.', true),
  ('Rodnar Shieldcrest', 'prf', 6, null, 'The opening goes straight at the one face that has not looked up yet.', true),
  ('Rodnar Shieldcrest', 'prf', 7, null, 'Rodnar lets the silence run a beat past comfortable, and steps into it.', true),
  ('Rodnar Shieldcrest', 'prf', 8, null, 'The rhythm sets first, in the boots, before anything else is offered.', true),
  ('Rodnar Shieldcrest', 'prf', 9, null, 'He gives them the shape of what is coming, then takes his time arriving there.', true),
  ('Rodnar Shieldcrest', 'prf', 10, null, 'Everything narrows to the space between his first breath and whatever the room decides to do with it.', true),
  ('Rodnar Shieldcrest', 'per', 1, null, 'Rodnar leads with the thing the other already believes, and builds outward from that stone.', true),
  ('Rodnar Shieldcrest', 'per', 2, null, 'The voice stays warm and unhurried, making room rather than pressing into it.', true),
  ('Rodnar Shieldcrest', 'per', 3, null, 'He puts the objection into words first, better than they could, and then answers it.', true),
  ('Rodnar Shieldcrest', 'per', 4, null, 'The case gets framed as something already half-agreed, wanting only a nod to finish.', true),
  ('Rodnar Shieldcrest', 'per', 5, null, 'He asks a question he knows the answer to, and lets them arrive there on their own feet.', true),
  ('Rodnar Shieldcrest', 'per', 6, null, 'Broad hands open, weight back, nothing about him crowding the space between.', true),
  ('Rodnar Shieldcrest', 'per', 7, null, 'Rodnar concedes the small point at once and entirely, and watches it buy him the larger one.', true),
  ('Rodnar Shieldcrest', 'per', 8, null, 'He makes the case plainly, once, and then stops talking and lets it sit.', true),
  ('Rodnar Shieldcrest', 'per', 9, null, 'He speaks to what they stand to lose, and never once to what he wants.', true),
  ('Rodnar Shieldcrest', 'per', 10, null, 'The tone drops into something confiding, as though Shayl had already settled the matter between them.', true),
  ('Rodnar Shieldcrest', 'rel', 1, null, 'The holy symbol comes into Rodnar''s hand and is gripped like a rail on a dark stair.', true),
  ('Rodnar Shieldcrest', 'rel', 2, null, 'He begins the rite from its opening words, letting the old shape of it steady him.', true),
  ('Rodnar Shieldcrest', 'rel', 3, null, 'The prayer to Shayl starts silent and only becomes sound halfway through.', true),
  ('Rodnar Shieldcrest', 'rel', 4, null, 'Rodnar kneels without hurry, the scale mail settling, and sets all of himself on what is owed here.', true),
  ('Rodnar Shieldcrest', 'rel', 5, null, 'The words come out in the order a young Unt''gar learned them, worn smooth from use.', true),
  ('Rodnar Shieldcrest', 'rel', 6, null, 'He holds the symbol out in front of him, arm straight, and does not waver.', true),
  ('Rodnar Shieldcrest', 'rel', 7, null, 'Doctrine arranges itself in his head — what is permitted, what is required, what is refused.', true),
  ('Rodnar Shieldcrest', 'rel', 8, null, 'He breathes the invocation out slowly, giving each of Shayl''s names its full weight.', true),
  ('Rodnar Shieldcrest', 'rel', 9, null, 'A thick hand comes up in the gesture his teachers made, before he understood it.', true),
  ('Rodnar Shieldcrest', 'rel', 10, null, 'Rodnar sets his faith down in front of the thing like a slab of stone, and stands behind it.', true),
  ('Rodnar Shieldcrest', 'slt', 1, null, 'One broad hand rises into plain view and does something worth watching. The other simply works.', true),
  ('Rodnar Shieldcrest', 'slt', 2, null, 'The fingers move once, briefly, in the moment those eyes are somewhere else.', true),
  ('Rodnar Shieldcrest', 'slt', 3, null, 'Rodnar keeps the conversation going with his face while his hand finishes.', true),
  ('Rodnar Shieldcrest', 'slt', 4, null, 'A pauldron turns to screen the movement, casual, exactly as it would have turned anyway.', true),
  ('Rodnar Shieldcrest', 'slt', 5, null, 'The thing is gone before the hand that took it has finished passing.', true),
  ('Rodnar Shieldcrest', 'slt', 6, null, 'He lets the noise of the room do half the work and supplies the rest with two thick fingers.', true),
  ('Rodnar Shieldcrest', 'slt', 7, null, 'The palm closes around it in the same motion that reaches past it.', true),
  ('Rodnar Shieldcrest', 'slt', 8, null, 'He sets the misdirection first and only then commits, unhurried, to the rest.', true),
  ('Rodnar Shieldcrest', 'slt', 9, null, 'Fingers walk the seam of the pocket, patient, taking the time not to snag.', true),
  ('Rodnar Shieldcrest', 'slt', 10, null, 'The whole of it happens inside one gesture that would have looked ordinary from any angle.', true),
  ('Rodnar Shieldcrest', 'ste', 1, null, 'Rodnar sets each boot down heel-last, testing the stone before he gives it his weight.', true),
  ('Rodnar Shieldcrest', 'ste', 2, null, 'Every scale of the mail gets stilled with a flat hand before the body moves at all.', true),
  ('Rodnar Shieldcrest', 'ste', 3, null, 'He breathes shallow through the beard and presses himself into the deepest part of the shadow.', true),
  ('Rodnar Shieldcrest', 'ste', 4, null, 'The route gets chosen for its footing rather than its length, and taken slowly.', true),
  ('Rodnar Shieldcrest', 'ste', 5, null, 'He waits out the noise of the place and travels only inside it.', true),
  ('Rodnar Shieldcrest', 'ste', 6, null, 'Rodnar flattens to the wall and slides along it, keeping his considerable outline inside its shape.', true),
  ('Rodnar Shieldcrest', 'ste', 7, null, 'A pause at the threshold — listening, counting, and only then crossing.', true),
  ('Rodnar Shieldcrest', 'ste', 8, null, 'The mace gets clamped against his side so it cannot speak.', true),
  ('Rodnar Shieldcrest', 'ste', 9, null, 'He moves when the light moves, and stops the instant it stops.', true),
  ('Rodnar Shieldcrest', 'ste', 10, null, 'Every step goes down as though the floor were owed an apology, and he goes on.', true),
  ('Rodnar Shieldcrest', 'sur', 1, null, 'Rodnar reads the trail for the freshest disturbance and follows it with his eyes before his boots.', true),
  ('Rodnar Shieldcrest', 'sur', 2, null, 'Wind, light and ground get weighed together into a single blunt guess about the hours ahead.', true),
  ('Rodnar Shieldcrest', 'sur', 3, null, 'He casts in a slow arc, hunting the place the tracks pick up again.', true),
  ('Rodnar Shieldcrest', 'sur', 4, null, 'A hand goes into the ash to learn whether it still holds warmth.', true),
  ('Rodnar Shieldcrest', 'sur', 5, null, 'He studies the treeline for the shape of the land hiding behind it.', true),
  ('Rodnar Shieldcrest', 'sur', 6, null, 'The kindling is built small and tight, sheltered from the wind by his own broad back.', true),
  ('Rodnar Shieldcrest', 'sur', 7, null, 'Rodnar picks the camp by what stands above it and behind it, not by how flat the ground lies.', true),
  ('Rodnar Shieldcrest', 'sur', 8, null, 'The water gets tasted upstream before anything else is decided.', true),
  ('Rodnar Shieldcrest', 'sur', 9, null, 'He sets the snare where the run narrows, working by the animal''s logic rather than his own.', true),
  ('Rodnar Shieldcrest', 'sur', 10, null, 'He reads the horizon for weather and says the verdict aloud, to nobody in particular.', true)
on conflict (pack, key, seq) where game_id is null do nothing;

insert into public.skill_prompts (key, game_id, label, facts, gear_tags, guidance) values
  ('acr', null, 'Acrobatics', array['ability','armor','wounds']::text[], '{}', 'Body control, balance, and momentum; worn armor makes every twist a negotiation with weight.'),
  ('ani', null, 'Animal Handling', array['ability','faith']::text[], array['feed','treat','ration']::text[], 'Patience and quiet presence; slow hands, steady voice, an offering if one is carried.'),
  ('arc', null, 'Arcana', array['ability','faith']::text[], array['book','scroll','tome']::text[], 'Scholarly recall — tracing remembered sigils, weighing what was read against what is seen.'),
  ('ath', null, 'Athletics', array['ability','armor','wounds']::text[], array['rope','piton','grapnel']::text[], 'Raw physical effort: muscle, grip, breath, and the dead weight of everything worn and carried.'),
  ('dec', null, 'Deception', array['ability','look']::text[], '{}', 'A controlled face and an easy voice; the lie worn like a second set of clothes.'),
  ('his', null, 'History', array['ability','faith']::text[], array['book','map']::text[], 'Memory reaching back — names, dates, the shape of old grudges and older treaties.'),
  ('ins', null, 'Insight', array['ability']::text[], '{}', 'Watching the eyes and the hands, listening past the words for what is actually being said.'),
  ('itm', null, 'Intimidation', array['ability','weapons','size','wounds']::text[], '{}', 'Menace through bearing: the deliberate stillness, the visible weapon, the promise of what comes next.'),
  ('inv', null, 'Investigation', array['ability']::text[], array['lamp','lantern','torch','glass']::text[], 'Methodical examination — corners, seams, the detail that does not belong.'),
  ('med', null, 'Medicine', array['ability','gear','faith']::text[], array['healer','bandage','herb','kit','salve']::text[], 'Practiced field hands and clear-eyed triage; faith and physic working the same wound.'),
  ('nat', null, 'Nature', array['ability']::text[], '{}', 'Reading the land like a text — growth, weather, spoor, and what disturbed them.'),
  ('prc', null, 'Perception', array['ability','senses']::text[], array['lamp','lantern','spyglass']::text[], 'Senses cast wide — the scan, the held breath, the discipline of noticing.'),
  ('prf', null, 'Performance', array['ability','faith','look']::text[], array['instrument','drum','flute']::text[], 'Voice and presence filling the space; the moment before an audience decides.'),
  ('per', null, 'Persuasion', array['ability','faith','look']::text[], '{}', 'Warmth and conviction; finding the argument the listener already half-believes.'),
  ('rel', null, 'Religion', array['ability','faith','gear']::text[], array['holy','symbol','prayer','incense','water']::text[], 'Rite and doctrine at the fingertips — the prayer half-spoken, the symbol gripped, the god remembered.'),
  ('slt', null, 'Sleight of Hand', array['ability','armor']::text[], array['tool','pick','wire']::text[], 'Quick fingers and misdirection; the hand the eye follows and the one it does not.'),
  ('ste', null, 'Stealth', array['ability','armor','wounds']::text[], array['cloak','boots','soft']::text[], 'Silence as a craft — placed feet, stilled breath, and the treachery of everything metal being worn.'),
  ('sur', null, 'Survival', array['ability','gear']::text[], array['rope','ration','tinder','blanket','flint']::text[], 'Trail-craft and weather-sense; the land read for food, shelter, direction, and danger.'),
  ('str_save', null, 'STR Save', array['ability','armor','wounds']::text[], '{}', 'Bracing against force — planted feet, locked grip, the body as a wall.'),
  ('dex_save', null, 'DEX Save', array['ability','armor','wounds']::text[], '{}', 'The instant of evasion — the flinch trained into a dive, armor dragging at the motion.'),
  ('con_save', null, 'CON Save', array['ability','wounds']::text[], '{}', 'Endurance from the marrow — gritting through what the body is being asked to survive.'),
  ('int_save', null, 'INT Save', array['ability','faith']::text[], '{}', 'The mind holding its own shape against intrusion — logic as a locked door.'),
  ('wis_save', null, 'WIS Save', array['ability','faith','gear']::text[], array['holy','symbol']::text[], 'Will as a bulwark — centering on what is known and holy while something pulls.'),
  ('cha_save', null, 'CHA Save', array['ability','faith']::text[], '{}', 'The self refusing to be rewritten — identity gripped like a handhold.'),
  ('str_check', null, 'STR Check', array['ability','armor','wounds']::text[], '{}', 'Plain application of strength — lift, shove, break, hold.'),
  ('dex_check', null, 'DEX Check', array['ability','armor']::text[], '{}', 'Precision of hand and body — the careful, exact motion.'),
  ('con_check', null, 'CON Check', array['ability','wounds']::text[], '{}', 'Sheer stamina — pushing the body past where it wants to stop.'),
  ('int_check', null, 'INT Check', array['ability']::text[], '{}', 'Working the problem — pattern, memory, deduction.'),
  ('wis_check', null, 'WIS Check', array['ability','faith']::text[], '{}', 'Attunement and judgment — sensing the shape of the situation.'),
  ('cha_check', null, 'CHA Check', array['ability','look']::text[], '{}', 'Force of presence — the room bending slightly toward the speaker.'),
  ('custom', null, '(custom rolls)', array['wounds']::text[], '{}', 'A moment of effort or fortune, grounded in who this character is.'),
  ('attack', null, 'Weapon Attack', array['ability','weapons','wounds']::text[], '{}', 'The committed strike — describe the weapon''s weight, arc, and intent; the body mechanics of a trained blow driven through wounds and fatigue.'),
  ('spell', null, 'Spellcasting', array['ability','faith','wounds']::text[], array['holy','symbol','prayer','incense']::text[], 'The faith made manifest — Shayl''s power moving through a working priest: mineral, patient, physical. Ground the magic in stone, crystal, ore, and craft imagery, never generic sparkle.'),
  ('death', null, 'Death Save', array['wounds','faith']::text[], array['holy','symbol']::text[], 'Narrate from the threshold — gray light, distant sounds, the stone-cold grip on life. Shayl weighing his servant. Never describe the outcome''s mechanics, only the fight to stay.')
on conflict (key) where game_id is null do nothing;

insert into public.spells (key, game_id, name, roll_name, level, cast_type, category, school, save_ability, dice, spell_atk, dc, concentration, ritual, range, duration, special_text, guidance, description) values
  ('sp_sacredflame', null, 'Sacred Flame', 'sacred flame', 0, 'Save', 'Cantrips', 'evocation', 'dex'::public.ability_code, '2d8', null, 15, false, false, '60 ft', 'Instant', 'Radiant damage on a failed DEX save; no damage on a success. The target gains no benefit from half or three-quarters cover against this save. (2d8 at level 5+; 3d8 at 11.)', 'Mineral radiance descends like light through a crystal seam — not thrown but called down, judgment arriving from directly above.', 'Flame-like radiance descends on a creature within range. The target must succeed on a Dexterity saving throw or take Radiant damage, gaining no benefit from half or three-quarters cover.'),
  ('sp_light', null, 'Light', 'light', 0, 'Utility', 'Cantrips', 'evocation', null, null, null, null, false, false, 'Touch', '1 hour', 'The touched object sheds Bright Light in a 20-ft radius and Dim Light 20 ft beyond, any color. Opaque covering blocks it.', 'A thumb pressed to stone or steel, and the glow of deep-earth crystal wakes within it — patient, cold, and steady.', 'You touch one Large or smaller object not worn or carried by someone else. Until the spell ends, it sheds Bright Light in a 20-foot radius and Dim Light for an additional 20 feet.'),
  ('sp_mending', null, 'Mending', 'mending', 0, 'Utility', 'Cantrips', 'transmutation', null, null, null, null, false, false, 'Touch', 'Instant', 'Repairs a single break or tear no larger than 1 foot, leaving no trace. Can repair a magic item physically but not restore its magic. Casting time 1 minute.', 'The craftsman-god''s smallest mercy — fingers tracing the fracture as the material remembers its wholeness and takes it back.', 'This spell repairs a single break or tear in an object you touch. As long as the damage is no larger than 1 foot in any dimension, you mend it, leaving no trace.'),
  ('sp_thaumaturgy', null, 'Thaumaturgy', 'thaumaturgy', 0, 'Utility', 'Cantrips', 'transmutation', null, null, null, null, false, false, '30 ft', 'Up to 1 min', 'Minor wonder: booming voice (Advantage on Intimidation), tremors in the ground, flames flicker or change color, doors fly open or slam, phantom sounds, altered eyes. Up to three 1-minute effects at once.', 'The mountain clearing its throat — a tremor underfoot, a voice with bedrock in it, the god of minerals reminding the room who owns the floor.', 'You manifest a minor wonder within range: booming voice, harmless ground tremors, flickering flames, phantom sounds, slamming doors, or altered eyes.'),
  ('sp_bless', null, 'Bless', 'bless', 1, 'Utility', '1st Circle', 'enchantment', null, null, null, null, true, false, '30 ft', '1 min', 'Up to three creatures each add 1d4 to attack rolls and saving throws until the spell ends. +1 target per slot level above 1.', 'Shayl''s favor laid over the party like veins of gold through granite — a litany spoken, three brows touched, the ore of courage seamed into them.', 'You bless up to three creatures within range. Whenever a target makes an attack roll or a saving throw before the spell ends, the target adds 1d4 to the attack roll or save.'),
  ('sp_curewounds', null, 'Cure Wounds', 'cure wounds', 1, 'Heal', '1st Circle', 'abjuration', null, '2d8+4', null, null, false, false, 'Touch', 'Instant', 'Healing at a level 1 slot. Cast higher for more: see Cure Wounds II / III.', 'Rough priest''s hands over the wound, a low hymn of setting and sealing — flesh knitting the way stone accepts mortar, layer by patient layer.', 'A creature you touch regains a number of Hit Points equal to 2d8 plus your spellcasting ability modifier. The healing increases by 2d8 for each spell slot level above 1.'),
  ('sp_curewounds2', null, 'Cure Wounds II', 'cure wounds 2', 2, 'Heal', '2nd Circle', 'abjuration', null, '4d8+4', null, null, false, false, 'Touch', 'Instant', 'Cure Wounds cast at a level 2 slot.', 'The deeper mending — both hands now, the hymn slower, the god''s attention drawn down into torn muscle and cracked bone.', 'Cure Wounds upcast: a creature you touch regains 4d8 plus your spellcasting ability modifier Hit Points.'),
  ('sp_curewounds3', null, 'Cure Wounds III', 'cure wounds 3', 3, 'Heal', '3rd Circle', 'abjuration', null, '6d8+4', null, null, false, false, 'Touch', 'Instant', 'Cure Wounds cast at a level 3 slot.', 'The great mending — forehead pressed to the wounded, the full weight of the mountain''s patience poured through one man''s hands.', 'Cure Wounds upcast: a creature you touch regains 6d8 plus your spellcasting ability modifier Hit Points.'),
  ('sp_shieldoffaith', null, 'Shield of Faith', 'shield of faith', 1, 'Utility', '1st Circle', 'abjuration', null, null, null, null, true, false, '60 ft', '10 min', 'One creature gains +2 AC for the duration. Bonus action to cast.', 'A shimmering lattice of crystal planes assembling around the chosen — Shayl''s geometry interposed between flesh and harm.', 'A shimmering field surrounds a creature of your choice within range, granting it a +2 bonus to AC for the duration.'),
  ('sp_prayerofhealing', null, 'Prayer of Healing', 'prayer of healing', 2, 'Heal', '2nd Circle', 'abjuration', null, '2d8', null, null, false, false, '30 ft', 'Instant', 'Up to five creatures who remain for the full 10-minute casting regain the rolled HP AND gain the benefits of a Short Rest. Each can''t benefit again until after a Long Rest.', 'The circle gathered, the long litany of quarry and forge — ten minutes of shared stillness while the god mends the whole company like one cracked wall.', 'Up to five creatures of your choice who remain within range for the spell''s entire casting gain the benefits of a Short Rest and regain 2d8 Hit Points. Casting time 10 minutes.'),
  ('sp_prayerofhealing2', null, 'Prayer of Healing II', 'prayer of healing 2', 3, 'Heal', '3rd Circle', 'abjuration', null, '3d8', null, null, false, false, '30 ft', 'Instant', 'Prayer of Healing at a level 3 slot: 3d8 HP plus Short Rest benefits for up to five creatures. 10-minute casting.', 'The long litany sung in the old tongue — deeper verses, older names of the god, the mending sunk further into every listener.', 'Prayer of Healing upcast: up to five creatures regain 3d8 Hit Points and gain Short Rest benefits.'),
  ('sp_lesserrestoration', null, 'Lesser Restoration', 'lesser restoration', 2, 'Utility', '2nd Circle', 'abjuration', null, null, null, null, false, false, 'Touch', 'Instant', 'End one condition on the touched creature: Blinded, Deafened, Paralyzed, or Poisoned. Bonus action.', 'Impurity drawn out of the body the way a smelter draws slag from ore — one touch, one word, the flaw skimmed away.', 'You touch a creature and end one condition on it: Blinded, Deafened, Paralyzed, or Poisoned.'),
  ('sp_silence', null, 'Silence', 'silence', 2, 'Utility', '2nd Circle', 'illusion', null, null, null, null, true, true, '120 ft', '10 min', '20-ft-radius sphere: no sound within or passing through. Inside: Immunity to Thunder damage, Deafened, and no Verbal spellcasting.', 'The hush of the deep places brought up into the air — a sphere of tomb-quiet where even the Deepsong itself would make no sound.', 'For the duration, no sound can be created within or pass through a 20-foot-radius Sphere centered on a point you choose within range. Verbal spellcasting is impossible there.'),
  ('sp_spiritualweapon', null, 'Spiritual Weapon', 'spiritual weapon', 2, 'Attack', '2nd Circle', 'evocation', null, '1d8+4', '+7', null, true, false, '60 ft', '1 min', 'A floating spectral weapon. Bonus action to cast and on later turns to move it 20 ft and attack again. Force damage.', 'A mace of translucent crystal assembling itself midair — Shayl''s own hand on a haft no one holds, swinging with the certainty of falling rock.', 'You create a floating, spectral force resembling a weapon. Make a melee spell attack against a creature within 5 feet of it; on a hit, the target takes 1d8 plus your spellcasting ability modifier Force damage. As a Bonus Action on later turns, move it up to 20 feet and repeat the attack.'),
  ('sp_wardingbond', null, 'Warding Bond', 'warding bond', 2, 'Utility', '2nd Circle', 'abjuration', null, null, null, null, false, false, 'Touch', '1 hour', 'Target gains +1 AC, +1 saves, and Resistance to all damage while within 60 ft — and each time it takes damage, you take the same amount. Requires paired platinum rings worn by both.', 'Two rings, one vein of ore — the priest binding his own body to another''s as load-bearing stone, taking the weight so the wall does not fall.', 'You touch a willing creature. While the target is within 60 feet, it gains +1 AC and saving throws and Resistance to all damage; each time it takes damage, you take the same amount.'),
  ('sp_zoneoftruth', null, 'Zone of Truth', 'zone of truth', 2, 'Save', '2nd Circle', 'enchantment', 'cha'::public.ability_code, null, null, 15, false, false, '60 ft', '10 min', '15-ft-radius sphere. Creatures entering or starting a turn there make a CHA save; on a failure they can''t speak a deliberate lie in the radius. You know who fails. The affected can be evasive but must be truthful.', 'Honesty enforced like a mason''s level laid across every word — the air itself refusing to carry a crooked sentence.', 'You create a magical zone that guards against deception in a 15-foot-radius Sphere. A creature that enters or starts its turn there makes a Charisma saving throw; on a failure it can''t speak a deliberate lie while in the radius. You know whether each creature succeeds or fails.'),
  ('sp_findtraps', null, 'Find Traps', 'find traps', 2, 'Utility', '2nd Circle', 'divination', null, null, null, null, false, false, '120 ft', 'Instant', 'Sense any trap within range and line of sight — presence and general nature, not exact location.', 'Palm flat to the ground, listening as stone tells on its makers — the hollow behind the wall, the tension in the hidden spring.', 'You sense any trap within range that is within line of sight. The spell reveals that a trap is present and its general nature, but not its location.'),
  ('sp_beaconofhope', null, 'Beacon of Hope', 'beacon of hope', 3, 'Utility', '3rd Circle', 'abjuration', null, null, null, null, true, false, '30 ft', '1 min', 'Any number of chosen creatures gain Advantage on WIS saves and Death Saves, and regain MAXIMUM Hit Points from any healing, for the duration.', 'The lamplight of the deep shrines kindled above the battlefield — despair finding no purchase, every mending striking true to its fullest.', 'Choose any number of creatures within range. For the duration, each target has Advantage on Wisdom saving throws and Death Saving Throws and regains the maximum number of Hit Points possible from any healing.'),
  ('sp_dispelmagic', null, 'Dispel Magic', 'dispel magic', 3, 'Utility', '3rd Circle', 'abjuration', null, null, null, null, false, false, '120 ft', 'Instant', 'End any spell of level 3 or lower on the target. For level 4+ spells: spellcasting ability check, DC 10 + spell level (Rodnar rolls d20+7).', 'The priest''s hand closing like a geode — foreign magic cracked from its housing and ground back into inert dust.', 'Choose one creature, object, or magical effect within range. Any ongoing spell of level 3 or lower on the target ends. For each spell of level 4 or higher, make a spellcasting ability check (DC 10 + the spell''s level) to end it.'),
  ('sp_fireball', null, 'Fireball', 'fireball', 3, 'Save', '3rd Circle', 'evocation', 'dex'::public.ability_code, '8d6', null, 15, false, false, '150 ft', 'Instant', '20-ft-radius sphere: 8d6 Fire on a failed DEX save, HALF on a success. Flammable objects ignite. +1d6 per slot level above 3.', 'Not Shayl''s usual gift — a stolen ember of the world''s molten heart, hurled and blooming into a sphere of magma-light and roar.', 'A bright streak flashes to a point you choose, then blossoms into a fiery explosion. Each creature in a 20-foot-radius Sphere makes a Dexterity saving throw, taking 8d6 Fire damage on a failed save or half as much on a successful one.'),
  ('sp_revivify', null, 'Revivify', 'revivify', 3, 'Utility', '3rd Circle', 'necromancy', null, null, null, null, false, false, 'Touch', 'Instant', 'A creature dead less than 1 minute revives with 1 HP. Consumes diamonds worth 300+ GP. Cannot restore missing parts or revive death by old age.', 'Diamonds crushed against the still chest, the god of what endures refusing this one subtraction — a spark struck back into cooling stone.', 'You touch a creature that has died within the last minute. That creature revives with 1 Hit Point. Consumes diamonds worth 300+ GP.'),
  ('sp_tongues', null, 'Tongues', 'tongues', 3, 'Utility', '3rd Circle', 'divination', null, null, null, null, false, false, 'Touch', '1 hour', 'The touched creature understands any spoken or signed language, and is understood by any creature that knows at least one language.', 'All speech ground down to its common bedrock — the touched one hearing every tongue as the same deep vein beneath different soils.', 'The creature you touch can understand any spoken or signed language it hears or sees, and when it communicates, any creature that knows at least one language can understand it.'),
  ('sp_deathward', null, 'Death Ward', 'death ward', 4, 'Utility', '4th Circle', 'abjuration', null, null, null, null, false, false, 'Touch', '8 hours', 'The first time the target would drop to 0 HP, it drops to 1 HP instead and the spell ends. An instant-death effect is negated instead.', 'A keystone set into the arch of a life — when everything else gives way, one stone holds, once, because the god placed it there.', 'You touch a creature. The first time it would drop to 0 Hit Points before the spell ends, it instead drops to 1 Hit Point, and the spell ends. An effect that would kill it instantly without damage is negated instead.'),
  ('sp_auraoflife', null, 'Aura of Life', 'aura of life', 4, 'Utility', '4th Circle', 'abjuration', null, null, null, null, true, false, 'Self', '10 min', '30-ft Emanation: you and allies have Resistance to Necrotic damage and HP maximums can''t be reduced. An ally at 0 HP starting its turn in the aura regains 1 HP.', 'The priest as living shrine — a radius of deep-earth warmth where death''s arithmetic is refused and the fallen are handed back their first heartbeat.', 'An aura radiates from you in a 30-foot Emanation. While in it, you and your allies have Resistance to Necrotic damage and your Hit Point maximums can''t be reduced. An ally with 0 Hit Points that starts its turn in the aura regains 1 Hit Point.')
on conflict (key) where game_id is null do nothing;

insert into public.techniques (key, game_id, name, roll_name, weapon, category, tier, min_level, dice, crit_min, fumble_max, special_text, guidance) values
  ('smash', null, 'Heavy Smash', 'heavy smash', 'mace of the deep song', 'Divine Strikes', 'Class 1', 1, '1d8', 20, 2, null, 'An overcommitted overhead swing — the mace raised high and brought down with crushing force, power traded for balance.'),
  ('swift', null, 'Swift Strike', 'swift strike', 'mace of the deep song', 'Divine Strikes', 'Class 1', 1, '1d6', 19, 1, null, 'A quick horizontal snap of the haft — speed and precision over power, the head singing through a tight arc.'),
  ('guard', null, 'Guard Break', 'guard break', 'mace of the deep song', 'Divine Strikes', 'Class 1', 2, '1d7', 20, 1, null, 'A calculated, angled strike meant to slip past shield rim and parry — patient, technical, inevitable.'),
  ('crushing', null, 'Crushing Hymn', 'crushing hymn', 'mace of the deep song', 'Divine Strikes', 'Class 1', 3, '1d8', 19, 1, null, 'Shayl''s litany chanted through the swing — the mace descending with divine weight, hymn and blow landing as one.'),
  ('judgment', null, 'Stone''s Judgment', 'stones judgment', 'mace of the deep song', 'Divine Strikes', 'Class 1', 4, '1d7', 20, 1, null, 'The mace ringing with mineral clarity — a measured, unyielding verdict delivered without flourish or doubt.'),
  ('earthshaker', null, 'Earthshaker', 'earthshaker', 'mace of the deep song', 'Divine Strikes', 'Class 1', 5, '1d10', 20, 2, null, 'The earth''s own fury channeled downward — a hard, committed swing that risks everything on the landing.'),
  ('resonance', null, 'Crystal Resonance', 'crystal resonance', 'mace of the deep song', 'Mineral Miracles', 'Class 2', 7, '1d12', 20, 3, null, 'The mace hums at a crystalline frequency, unstable harmonics building through the swing — devastating if it lands true.'),
  ('ward', null, 'Mineral Ward', 'mineral ward', 'mace of the deep song', 'Mineral Miracles', 'Class 2', 5, '1d4', 20, 1, '🛡️ Mineral Ward: crystalline energy coalesces into armor — gain temporary HP equal to 1d6 + WIS mod (roll it). The blessing of Shayl protects the faithful.', 'A protective sweeping arc — mineral energy crystallizing along the path of the mace into gleaming temporary armor.'),
  ('deepsong', null, 'Deepsong Echo', 'deepsong echo', 'mace of the deep song', 'Mineral Miracles', 'Class 2', 6, '1d14', 18, 0, null, 'A strike at perfect pitch — the mace resonating with the deep song of ancient stone, the frequency seeking hidden weakness.'),
  ('petrify', null, 'Petrifying Touch', 'petrifying touch', 'mace of the deep song', 'Mineral Miracles', 'Class 2', 9, '1d10', 20, 1, '🗿 Petrifying Touch: target makes a STR save (DC 8 + prof + WIS mod) or is restrained for 1+1d4 rounds as mineral essence calcifies flesh. Restrained enemies grant advantage.', 'Mineral essence infused through the strike — where the mace lands, flesh begins to stiffen and calcify.'),
  ('hc_steelcone', null, 'Steel Cone Bolt', 'steel cone bolt', 'heavy crossbow', 'Siege Bolts', 'Class 1', 1, '1d12', 19, 1, '⚔️ Steel Cone: hardened armor-piercing tip — treat the target''s armor as 3 AC lower. ⏳ Reload: 2 full rounds; cannot reload while moving.', 'The heavy draw cranked back tooth by tooth, the cone-tipped bolt seated, breath held through the deliberate aim — mechanical power waiting for release.'),
  ('hc_weighted', null, 'Weighted Cone Bolt', 'weighted cone bolt', 'heavy crossbow', 'Siege Bolts', 'Class 1', 2, '3d6', 20, 1, '⚔️⚔️ Weighted Cone: reinforced tip — treat the target''s armor as 5 AC lower; punches through plate and shield alike. Reduced range (140/560). ⏳ Reload: 2 full rounds.', 'An overweight bolt that drops the crossbow''s aim point — the shot fired flatter and closer, trading reach for a strike that ignores steel entirely.'),
  ('hc_broadhead', null, 'Broadhead Bolt', 'broadhead bolt', 'heavy crossbow', 'Siege Bolts', 'Class 1', 1, '1d10', 20, 1, '🩸 Broadhead: wide cutting head — target bleeds 1d6 per turn (2d6 on a crit) until treated. Best against unarmored targets. ⏳ Reload: 2 full rounds.', 'A wide-bladed head meant for flesh, not steel — the siege draw burying it deep enough that the wound will not close on its own.'),
  ('hc_barbed', null, 'Barbed Cone Bolt', 'barbed cone bolt', 'heavy crossbow', 'Siege Bolts', 'Class 1', 3, '1d10', 20, 1, '🩸⚔️ Barbed Cone: target bleeds 1d6 per turn; removal takes an action and a DC 13 Medicine or DC 15 STR check — failure deals 1d6. On a crit: 1d8 bleed, DC 15/18, failed removal 2d6.', 'Reversed barbs behind the cone tip — a bolt designed to stay where it lands, every movement of the target working it deeper.'),
  ('hc_signal', null, 'Signal Bolt', 'signal bolt', 'heavy crossbow', 'Siege Bolts', 'Class 1', 1, '1d8', 20, 1, '📏🔊 Signal Bolt: extreme range (200/800 ft) — whistle audible to 1000 ft, smoke trail visible for 1 minute. Ranging shots and signaling allies. ⏳ Reload: 2 full rounds.', 'A hollow lightweight bolt loosed high and far — the shot less about the target than the shrieking whistle and smoke line it draws across the sky.'),
  ('hc_explosive', null, 'Explosive Bolt', 'explosive bolt', 'heavy crossbow', 'Siege Bolts', 'Class 2', 5, '1d12', 20, 1, '💥 Explosive Bolt: alchemical warhead — on a hit, all creatures within 5 ft take 2d8 fire (DEX save DC 13 + prof for half). Reduced range (140/560). Expensive and rare. ⏳ Reload: 2 full rounds.', 'A hollow bolt packed with volatile compound — the shot deliberate and slightly high, aimed less at a man than at the space a group of them shares.'),
  ('hc_devastating', null, 'Devastating Impact', 'devastating impact', 'heavy crossbow', 'Siege Bolts', 'Class 2', 7, '3d10', 19, 1, '💥 Devastating Impact: the crossbow drawn past its rated tension for a maximum-power shot. ⏳ Reload: 2 full rounds — the mechanism must be inspected after.', 'The windlass cranked past the last tooth, the stock braced hard against shoulder and stone — a shot with the full fury of the siege engine behind it.'),
  ('hc_suppress', null, 'Suppressing Shot', 'suppressing shot', 'heavy crossbow', 'Siege Bolts', 'Class 2', 4, '1d12', 20, 1, '🎯 Suppressing Shot: the target makes a WIS save (DC 13 + prof) or is frightened until the end of their next turn — disadvantage on checks and attacks while they cower. ⏳ Reload: 2 full rounds.', 'A shot placed to terrify rather than kill — splintering stone beside the target''s head, the crack of impact announcing what the next bolt will do.'),
  ('hm_wristsnap', null, 'Wrist Snap', 'wrist snap', 'light hammer', 'Hammer Melee', 'Class 1', 1, '1d4', 19, 1, null, 'All wrist, no arm — a short flicking strike faster than the eye tracks, the small head darting at fingers, nose, or temple.'),
  ('hm_haftjab', null, 'Haft Jab', 'haft jab', 'light hammer', 'Hammer Melee', 'Class 1', 1, '1d4', 20, 1, null, 'The butt of the handle driven straight in like a short spear — ugly, close, and impossible to parry with anything graceful.'),
  ('hm_roundstrike', null, 'Round Strike', 'round strike', 'light hammer', 'Hammer Melee', 'Class 1', 2, '1d6', 20, 1, null, 'The workhorse blow — a full circular swing from the shoulder, hips turning through it, the hammer arriving with the whole body behind it.'),
  ('hm_downbeat', null, 'Downbeat', 'downbeat', 'light hammer', 'Hammer Melee', 'Class 1', 3, '1d6', 19, 2, null, 'A committed vertical drop onto collarbone or guard — everything spent on one falling beat, nothing held back for recovery.'),
  ('hm_knucklebreaker', null, 'Knuckle Breaker', 'knuckle breaker', 'light hammer', 'Hammer Melee', 'Class 2', 4, '1d4', 20, 1, '🖐️ Knuckle Breaker: a strike to the weapon hand — the target makes a DEX save (DC 8 + prof + STR mod) or drops what they are holding.', 'Not the man — the grip. The hammer head rapped hard across knuckles and thumb, aimed at making a hand forget its business.'),
  ('hm_kneecapper', null, 'Kneecapper', 'kneecapper', 'light hammer', 'Hammer Melee', 'Class 2', 5, '1d6', 20, 1, '🦵 Kneecapper: a low strike to knee or ankle — the target makes a CON save (DC 8 + prof + STR mod) or their speed is halved until the end of their next turn.', 'A crouching lateral blow at the joint that carries the weight — the strike a mason uses on stone he wants to crack, not shatter.'),
  ('hm_rhythm', null, 'Rhythm of Blows', 'rhythm of blows', 'light hammer', 'Hammer Melee', 'Class 2', 6, '1d8', 19, 2, null, 'The hammer falling in working tempo — strike, recover, strike — each beat borrowing speed from the last, the way a smith works hot iron.'),
  ('ht_straightcast', null, 'Straight Cast', 'straight cast', 'light hammer (thrown)', 'Hammer Thrown', 'Class 1', 1, '1d4', 20, 1, null, 'The plain honest throw — one step, a flat release, the hammer spinning once and arriving head-first. Nothing clever; clever misses.'),
  ('ht_shortlob', null, 'Short Lob', 'short lob', 'light hammer (thrown)', 'Hammer Thrown', 'Class 1', 2, '1d4', 19, 1, null, 'A close-range underhand toss with almost no spin — placed rather than hurled, dropped onto helm or shoulder from a pace and a half away.'),
  ('ht_tumbling', null, 'Tumbling Throw', 'tumbling throw', 'light hammer (thrown)', 'Hammer Thrown', 'Class 1', 3, '1d6', 20, 2, null, 'Hurled end over end with the full arm and a step behind it — more rotation, more force, and more ways for the handle to arrive instead of the head.'),
  ('ht_deflectingcast', null, 'Deflecting Cast', 'deflecting cast', 'light hammer (thrown)', 'Hammer Thrown', 'Class 2', 4, '1d4', 20, 1, '🛡️ Deflecting Cast: thrown at the target''s weapon arm mid-swing — the target has disadvantage on their next attack roll before the end of their next turn.', 'A throw timed against the target''s own wind-up — the hammer meeting the raised arm at the top of its arc, spoiling the blow before it starts.'),
  ('ht_felltherunner', null, 'Fell the Runner', 'fell the runner', 'light hammer (thrown)', 'Hammer Thrown', 'Class 2', 5, '1d4', 20, 1, '🏃 Fell the Runner: a low spinning throw at shin height — the target makes a DEX save (DC 8 + prof + STR mod) or falls prone.', 'Thrown low and skimming, handle whirling flat like a scythe at ankle height — a throw for the fleeing back, the charging line, the moment legs matter most.'),
  ('ht_twinrebound', null, 'Twin Rebound', 'twin rebound', 'light hammer (thrown)', 'Hammer Thrown', 'Class 2', 6, '1d6', 19, 2, null, 'Banked off wall, post, or shield boss — the hammer arriving from the flank while every eye is on the thrower''s empty hand. Geometry as a weapon.')
on conflict (key) where game_id is null do nothing;
