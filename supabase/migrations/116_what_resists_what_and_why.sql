-- 116. WHAT HURTS SOMEBODY LESS, MORE, OR NOT AT ALL.
--
-- `species.damage_resistances` has existed since 056 and the sheet
-- printed it followed by "(DM applies)" - an honest label for a thing
-- that does nothing. The Unt'garoth have been resistant to fire and
-- cold on paper for a month and no number ever moved.
--
-- ONE VOCABULARY, which is 100's. A resistance is a GRANT:
-- `resist.fire`, `immune.poison`, `vulnerable.cold`. That buys the
-- three remaining sources at once - an item, a spell's effect and a
-- class feature all already carry grants or are about to - rather than
-- a resistance column on each of four tables that could disagree.
--
-- SEVERAL TYPES IN ONE TARGET IS A CHOICE. Protection from Energy is
-- "choose one of acid, cold, fire, lightning, or thunder", so
-- `resist.acid|cold|fire|lightning|thunder` is one grant that asks a
-- question, settled when the spell is cast. Five separate spells or a
-- resistance to all five would both have been wrong.
--
-- A RESISTANCE CARRIES NO VALUE. Being resistant is the whole of what
-- it says; a `1` beside it would be a number somebody eventually tries
-- to add to something.

-- ---------------------------------------------------------------------
-- A CLASS FEATURE CAN NOW DO SOMETHING
-- ---------------------------------------------------------------------
-- 087 gave a feature a name, a level, a choice and a use count, which
-- is everything except an effect. Rage has read "resistance to
-- bludgeoning, piercing, and slashing damage" in its own text column
-- since then with nothing able to act on it.
alter table public.class_features
  add column if not exists grants jsonb not null default '[]'::jsonb;

comment on column public.class_features.grants is
  '100''s grant vocabulary: what this feature does to whoever holds it. '
  'A feature with no `uses` grants these PASSIVELY - Purity of Body is '
  'simply true. A feature WITH `uses` grants them only while its effect '
  'is running, because Rage is something you spend rather than something '
  'you are.';

-- ---------------------------------------------------------------------
-- THE TWO BASE-CLASS FEATURES THAT GRANT ONE
-- ---------------------------------------------------------------------
-- Subclasses are where 5e keeps most of its resistances (Draconic
-- Ancestry, Elemental Adept, the Oath auras) and none of them are
-- seeded yet. These two are base class features and both already exist.

-- Rage: spent, so these are live only while the effect runs.
update public.class_features
   set grants = '[{"target":"resist.bludgeoning","source":"Rage"},
                  {"target":"resist.piercing","source":"Rage"},
                  {"target":"resist.slashing","source":"Rage"}]'::jsonb
 where class_key = 'barbarian' and key = 'rage';

-- Purity of Body: no uses, so simply true from level 10 onward.
-- IMMUNITY RATHER THAN RESISTANCE, which is what the book says and is
-- a different kind of fact: half of a large number still kills.
update public.class_features
   set grants = '[{"target":"immune.poison","source":"Purity of Body"}]'::jsonb
 where class_key = 'monk' and key = 'purity_of_body';

-- ---------------------------------------------------------------------
-- THE CLERIC SPELLS THAT GRANT ONE
-- ---------------------------------------------------------------------
-- 114 gave `spells.grants` and filled in the ones that add a die. These
-- are the ones that change what damage costs.

-- Protection from Poison, 1 hour, no concentration. The spell also
-- gives advantage on saves against poison, which is not a number and is
-- not expressible here - the description says so and a DM reads it.
update public.spells
   set grants = '[{"target":"resist.poison","source":"Protection from Poison"}]'::jsonb
 where key = 'sp_protectionfrompoison';

-- Protection from Energy. THE ONE THAT ASKS A QUESTION.
update public.spells
   set grants = '[{"target":"resist.acid|cold|fire|lightning|thunder",
                   "source":"Protection from Energy"}]'::jsonb
 where key = 'sp_protectionfromenergy';

-- Aura of Life: resistance to necrotic.
--
-- APPLIED TO WHOEVER IT IS CAST ON, which is a simplification stated
-- rather than hidden. The spell is an aura around the caster that
-- protects everyone in 30 feet, and this engine has no aura - so a
-- cleric casts it at each ally they mean to cover. That is fewer
-- creatures than the spell protects and never more, which is the safe
-- direction to be wrong in.
update public.spells
   set grants = '[{"target":"resist.necrotic","source":"Aura of Life"}]'::jsonb
 where key = 'sp_auraoflife';

-- NOT HEROES' FEAST, deliberately. The spell grants immunity to poison
-- for 24 hours, but its `duration` is "Instantaneous" - which is
-- correct per the book, where the lasting part is described in the text
-- rather than in the duration line. `spellcast::lasts` reads the
-- duration and would make no effect at all, so a grant here would be a
-- third silent no-op of exactly the kind this migration exists to end.
-- It needs a separate column saying how long the BENEFIT lasts.
