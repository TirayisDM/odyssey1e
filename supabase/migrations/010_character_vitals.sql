-- =====================================================================
-- 010_character_vitals.sql
-- odyssey1e — a character can be hit
-- =====================================================================
--
-- Everything so far treats a character as a thing that rolls. To be a
-- TARGET it needs two numbers it has never had: how hard it is to hit,
-- and how much damage it can take. This adds the vitals; the dossier
-- (faith, appearance, senses, ideal, bond) is deliberately left in the
-- export until the narrator that wants it exists.
--
-- AC IS COMPUTED, AND THE EXPORT WILL LIE TO YOU ABOUT IT.
--
-- The Foundry export reads {"calc": "default", "flat": 14} and the
-- spreadsheet dutifully has an AC_Flat column saying 14. Fourteen is
-- NOT Rodnar's armour class. `calc: "default"` means Foundry computes
-- AC from equipped armour, and `flat` is a slot consulted only when
-- calc is "flat" - so 14 is a leftover in a field nobody reads.
--
-- His actual AC is 15: Scale Mail's base_ac of 14 plus a DEX modifier
-- of +1, capped at the armour's dex_cap of 2. Copying AC_Flat into this
-- table would have had every attack on him resolve against the wrong
-- number by one, permanently, with nothing anywhere to suggest it.
--
-- So ac_override is NOT "the AC". It is the flat value used only when
-- ac_mode is 'flat', which is the escape hatch for a monster or a magic
-- effect that sets AC outright. The ordinary path computes, in
-- equipment.rs, from items.base_ac and items.dex_cap - the two columns
-- 008 stored and described as "unused until something computes AC".
--
-- HP_MAX IS PORTED; CURRENT HP IS NOT.
--
-- The export says hp.value 45, hp.max 74. The spreadsheet's HP_Current
-- column says 74. They disagree, and neither is authoritative about
-- right now - a current HP copied out of a months-old export is a
-- snapshot of a fight that ended long ago.
--
-- So only the max is a fact worth storing. Current HP will be max minus
-- the sum of the damage events, once those exist, which sidesteps the
-- disagreement and keeps the event log honest from its first row. A
-- character with no events is at full health, which is the right answer
-- for every character in the database today.
--
-- WHAT HAS NO CONSUMER YET, AND IS HERE ANYWAY.
--
-- death_successes, death_failures and exhaustion are counters the rules
-- modules will read when they are ported. They are three small columns
-- on a row that is being altered regardless, they came from the same
-- export in the same pass, and leaving them out would mean coming back
-- for them. That is a different judgement from the dossier, which is
-- thirty columns serving a narrator that does not exist.
-- =====================================================================

alter table public.characters add column hp_max          integer;
alter table public.characters add column hp_temp         integer not null default 0;
alter table public.characters add column hp_temp_max     integer not null default 0;
alter table public.characters add column ac_mode         text not null default 'default';
alter table public.characters add column ac_override     integer;
alter table public.characters add column death_successes smallint not null default 0;
alter table public.characters add column death_failures  smallint not null default 0;
alter table public.characters add column exhaustion      smallint not null default 0;
alter table public.characters add column inspiration     boolean not null default false;
alter table public.characters add column size            text;

comment on column public.characters.hp_max is
  'Maximum hit points. The only HP fact stored: current HP is max less the sum of the damage events, so a character with no events is at full health. NULL means never set - a character that cannot yet be meaningfully attacked.';
comment on column public.characters.hp_temp is
  'Temporary hit points. Absorbed before real HP and not healed back; zero, never NULL.';
comment on column public.characters.hp_temp_max is
  'The temp HP pool''s own ceiling, from the export. Carried for parity; no rule reads it yet.';
comment on column public.characters.ac_mode is
  'default computes AC from equipped armour, in equipment.rs. flat uses ac_override and ignores what is worn. The export''s calc field, same vocabulary.';
comment on column public.characters.ac_override is
  'THE FLAT AC, AND ONLY WHEN ac_mode IS flat. Not "the AC" - the export carries flat 14 for a character whose real AC is 15, because his calc is default and the field is unread. Required when the mode is flat, forbidden from meaning anything when it is not.';
comment on column public.characters.death_successes is
  'Death saving throw successes, 0-3. No rule reads it yet; the death module is unported.';
comment on column public.characters.death_failures is
  'Death saving throw failures, 0-3. No rule reads it yet.';
comment on column public.characters.exhaustion is
  'Exhaustion level, 0-6. No rule reads it yet.';
comment on column public.characters.size is
  'tiny sm med lg huge grg, as the export spells it. Rodnar is med. Wanted by the narrator''s size section before any rule needs it.';

-- A flat AC needs a number to be flat AT. Expressible here, so the
-- Rust never has to handle a mode whose value is missing.
alter table public.characters
  add constraint characters_ac_mode_check
  check (ac_mode in ('default', 'flat'));

alter table public.characters
  add constraint characters_flat_ac_has_a_value_check
  check (ac_mode <> 'flat' or ac_override is not null);

alter table public.characters
  add constraint characters_hp_max_check
  check (hp_max is null or hp_max >= 1);

alter table public.characters
  add constraint characters_hp_temp_check
  check (hp_temp >= 0 and hp_temp_max >= 0);

-- 5e caps all three; a counter past its ceiling is a bug upstream.
alter table public.characters
  add constraint characters_death_saves_check
  check (death_successes between 0 and 3 and death_failures between 0 and 3);

alter table public.characters
  add constraint characters_exhaustion_check
  check (exhaustion between 0 and 6);

alter table public.characters
  add constraint characters_size_check
  check (size is null or size in ('tiny','sm','med','lg','huge','grg'));

-- The twins, from the same export 008 seeded items out of. Matched by
-- name, not by a generated id; a no-op anywhere those rows do not
-- exist. ac_mode stays 'default' deliberately - the point of this
-- migration is that his AC is computed, and comes out 15, not 14.
update public.characters
   set hp_max = 74,
       size   = 'med'
 where name in ('Character1', 'Character2');
