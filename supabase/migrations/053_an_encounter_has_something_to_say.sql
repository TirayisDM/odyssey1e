-- =====================================================================
-- 053_an_encounter_has_something_to_say.sql
-- odyssey1e — a fight has a description, and is retired rather than deleted
-- =====================================================================
--
-- An encounter is a name, a status and a place. Everything a DM knows
-- about it - what the room looks like, what the goblins want, what
-- happens when the door opens - lives in their head or on paper beside
-- the screen.
--
-- ---------------------------------------------------------------------
-- NARRATIVE IS NOT `narrative_lines`, AND THE NAME COLLISION IS WORTH
-- CLEARING UP BEFORE IT CONFUSES SOMEBODY
-- ---------------------------------------------------------------------
--
-- 006 seeded `narrative_lines`: flavour the ENGINE picks from when it
-- describes a roll - a crit with a mace reads differently from a
-- fumble with a bow. Those are generated, chosen at resolve time, and
-- belong to the dice.
--
-- This is the opposite kind of text. A human wrote it, it is about ONE
-- encounter, and nothing selects it - it is read aloud or referred to.
-- The engine will never touch this column, which is exactly the
-- distinction 043 drew about `special_text`: prose the engine does not
-- read is still worth storing, as long as nobody expects it to fire.
--
-- ---------------------------------------------------------------------
-- ARCHIVED IS NOT A STATUS, AND THAT IS DELIBERATE
-- ---------------------------------------------------------------------
--
-- `status` is already draft / active / ended / cancelled, and 034 was
-- careful about the last two: ENDED is what will trigger the experience
-- review and the journal entry, and CANCELLED must award nothing. Those
-- four are about what HAPPENED in the fiction.
--
-- Being retired from the list is not something that happened in the
-- fiction. It is a DM tidying their screen, and folding it into
-- `status` would either invent a fifth word that means "please stop
-- showing me this" - which 034's XP review would then have to learn to
-- ignore - or overload `cancelled`, which already means something a
-- player can feel.
--
-- So it is a separate boolean, and the two are read together: a
-- cancelled encounter can be archived, and so can an ended one, and
-- archiving neither cancels nor ends anything.
--
-- DELETION IS NOT OFFERED, which is the whole point of the column.
-- Rolls point at an encounter, actions point at rolls, and hit point
-- events point at the characters who were in it. Deleting a fight
-- would either cascade through a session's history or be refused by
-- the foreign keys - and the refusal is the honest answer, so the
-- screen should not ask the question. 011 made the same call about an
-- actor that has already rolled.
--
-- THE PARTIAL UNIQUE INDEX ON ACTIVE IS UNTOUCHED. One live encounter
-- per game is still the rule; an archived one that is somehow still
-- `active` would still hold that slot, which is correct - archiving is
-- a view preference and must not quietly change what is in front of
-- the table.
-- =====================================================================

alter table public.encounters add column narrative text;

alter table public.encounters
  add column archived boolean not null default false;

comment on column public.encounters.narrative is
  'What this encounter IS, written by a human and read by one. NOT narrative_lines, which is flavour the engine picks from at resolve time - nothing selects this, nothing generates it, and the engine will never touch it. The same kind of text as techniques.special_text: prose worth storing precisely because nobody expects it to fire.';
comment on column public.encounters.archived is
  'Retired from the list. NOT A STATUS, deliberately: status says what happened in the fiction - draft, active, ended, cancelled - and 034 gave ended and cancelled meanings the future XP review depends on. Being tidied off a screen is not something that happened in the fiction. Read together with status, so a cancelled encounter can be archived and archiving cancels nothing. There is no delete: rolls point at encounters and actions point at rolls, so removing one would either cascade through a session or be refused - and the refusal is the honest answer, so the screen does not ask.';

-- The list reads unarchived-first and by age, which is the order the
-- screen wants and the index it will use.
create index encounters_shelf_idx
  on public.encounters(game_id, archived, created_at desc);
