-- =====================================================================
-- 100_what_a_magic_thing_grants.sql
-- odyssey1e — enchantment, as one vocabulary everything can read
-- =====================================================================
--
-- 049 gave an OBJECT seven overrides and they all answer the same kind
-- of question: what is this thing. A greatsword that is unusually
-- large, a mace that does 1d8 instead of 1d6. None of them touches the
-- person holding it, and that is the whole of what magic does.
--
-- A +1 longsword is not a longsword with a bigger die. It is a
-- longsword that makes its WIELDER better, and nothing in this schema
-- could say so.
--
-- ---------------------------------------------------------------------
-- ONE VOCABULARY, READ BY EVERY CONSUMER
-- ---------------------------------------------------------------------
--
-- The temptation is a column per effect: `attack_bonus`, `ac_bonus`,
-- `str_bonus`. Six of those is a migration every time somebody invents
-- an item, and six readers that each know a different subset.
--
-- A GRANT IS A ROW IN A LIST: what it touches, how, and by how much.
--
--   {"target": "ac",        "mode": "add", "value": 1}
--   {"target": "attack",    "mode": "add", "value": 1}
--   {"target": "damage",    "mode": "add", "value": 1}
--   {"target": "str",       "mode": "set", "value": 19}
--   {"target": "save",      "mode": "add", "value": 1}
--   {"target": "save.dex",  "mode": "add", "value": 1}
--   {"target": "skill.ste", "mode": "add", "value": 2}
--
-- grants.rs is the only thing that parses this, every consumer asks it
-- the same question, and the fourth source of bonuses - a class
-- feature, a curse, a potion - costs nothing new.
--
-- THE SAME SHAPE 094 GAVE AN EFFECT, on purpose. An enchanted item is
-- something that is true for a while; the while is "as long as you are
-- wearing it" rather than a tick deadline. When effects start granting
-- numbers they produce this structure and the resolver cannot tell
-- where it came from.
--
-- ---------------------------------------------------------------------
-- ADD AND SET ARE DIFFERENT RULES, NOT TWO WORDS
-- ---------------------------------------------------------------------
--
--   add   signed and CUMULATIVE. Two +1 cloaks are +2. A cursed -2 is
--         an add, which is why the value is signed and why
--         AbilitySource has said "negative is as real as positive"
--         since 056.
--   set   a FLOOR, not an overwrite, which is how 5e words every item
--         that uses it: "your Strength is 19 unless it is already 19
--         or higher". Two sets do not stack - the higher wins - and a
--         set below what somebody already has does nothing at all.
--
-- A set that could LOWER a score is deliberately not expressible. That
-- is a curse, it is an `add` with a negative value, and keeping the
-- two apart means a Belt of Giant Strength can never accidentally nerf
-- somebody who was already stronger than it.
--
-- ---------------------------------------------------------------------
-- MAGIC IGNORES THE NATURAL CEILING, WHICH IS THE POINT
-- ---------------------------------------------------------------------
--
-- 099 stopped a Ny'ook writing a Strength above 13, and the rule Dave
-- stated in the same breath was that a spell or an item MAY carry them
-- over it. So a grant is applied AFTER the cap and is not subject to
-- it: natural score, then species, then improvements, all capped - and
-- then the magic, on top.
--
-- ---------------------------------------------------------------------
-- TWO PLACES, BECAUSE THERE ARE TWO KINDS OF MAGIC ITEM
-- ---------------------------------------------------------------------
--
--   items.grants     the CATALOGUE kind. A Cloak of Protection is a
--                    type of thing, stocked and bought and owned by
--                    several people, and every one of them is +1.
--   objects.grants   THIS one. The sword a smith enchanted for Falon
--                    is one sword, and 049 already chose the object
--                    as the place a particular thing differs.
--
-- They are collected together and both apply. An object's grants do
-- not REPLACE its type's - a +1 longsword that somebody further
-- enchanted is +1 from the catalogue and whatever else from the row -
-- which is the opposite of how 049's overrides behave, and right for
-- the opposite reason: an override answers "what is this", where only
-- one answer can be true, and a grant answers "what does this give",
-- where several can.
--
-- ---------------------------------------------------------------------
-- WHEN A GRANT APPLIES
-- ---------------------------------------------------------------------
--
-- While the object is IN A SLOT on the character - worn or held. A +1
-- sword in a backpack makes nobody better, and 084 already made `slot`
-- the one place that answers whether a thing is on somebody.
--
-- A grant may add "needs_attunement": true, and then it also waits for
-- `objects.attuned`. Attunement already exists with 5e's limit of
-- three enforced in carry.rs, so this is the gate rather than a new
-- one. Default false: plenty of magic needs no attunement, and a
-- default that silently switched items off would read as the grants
-- not working.
-- =====================================================================

alter table public.items
  add column if not exists grants jsonb not null default '[]'::jsonb;

alter table public.objects
  add column if not exists grants jsonb not null default '[]'::jsonb;

comment on column public.items.grants is
  'What this KIND of thing gives whoever wears or holds it, as a list of {target, mode, value}. The catalogue half of enchantment: a Cloak of Protection is +1 for everybody who owns one. Parsed only by grants.rs; every consumer asks that rather than reading this.';

comment on column public.objects.grants is
  'What THIS particular thing gives, on top of whatever its type already grants. The individual half of enchantment - the sword a smith worked on, rather than a kind of sword. Added to the type''s grants rather than replacing them, which is the opposite of how 049''s overrides behave and right for the opposite reason: an override says what something IS and only one answer can be true, a grant says what it GIVES and several can.';

-- A grant list is small and read whole every time; what is worth an
-- index is finding the magic at all.
create index if not exists items_that_grant
  on public.items ((grants <> '[]'::jsonb)) where grants <> '[]'::jsonb;
create index if not exists objects_that_grant
  on public.objects ((grants <> '[]'::jsonb)) where grants <> '[]'::jsonb;
