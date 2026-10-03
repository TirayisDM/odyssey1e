-- 114. WHAT A SPELL GRANTS WHILE IT LASTS.
--
-- 112 made Bless land on Falon and sit there for a minute. Nothing read
-- it, so his attack rolls were unchanged - an effect with a d4 on it
-- and a d20 that never saw the d4.
--
-- THE SAME VOCABULARY AS AN ENCHANTMENT, which is the whole point. 100
-- and grants.rs already answer "what does this thing do to the person
-- it is attached to" for a magic item, in one shape every consumer
-- asks about:
--
--   {"target": "attack", "mode": "add", "dice": "1d4"}
--
-- A spell is the same question with a clock on it. Giving effects their
-- own vocabulary would have been a second set of words for one idea,
-- and a second set of readers to keep in step.
--
-- `dice` IS 114's ADDITION to that vocabulary. Bless is +1d4, not +2 -
-- and a shape that could only say "+2" would have forced either an
-- average nobody agreed to or a parallel mechanism for spells.
--
-- ON THE CATALOGUE AND ON THE EFFECT, both. The spell says what it
-- does; casting COPIES that onto the effect, so a DM who retunes Bless
-- next month does not silently change what is already running on
-- somebody. The same snapshot rule 001 applies to a roll's names.
--
-- SEE 115: two of the five seeded here were wrong and are corrected
-- there rather than edited out of this file.

alter table spells  add column if not exists grants jsonb not null default '[]'::jsonb;
alter table effects add column if not exists grants jsonb not null default '[]'::jsonb;

comment on column spells.grants is
  'What this spell does to whoever it is on, while it lasts, in grants.rs''s vocabulary - the same one magic items use: [{"target":"attack","mode":"add","dice":"1d4"}]. Empty for most spells, which do their work by being read rather than by moving a number.';
comment on column effects.grants is
  'What this effect adds to the rolls of whoever carries it. COPIED FROM THE SPELL AT CASTING rather than looked up - a DM retuning Bless next month must not silently change what is already running on somebody, which is 001''s snapshot rule.';

update spells set grants = '[
  {"target":"attack","mode":"add","dice":"1d4","source":"Bless"},
  {"target":"save","mode":"add","dice":"1d4","source":"Bless"}
]'::jsonb where key = 'sp_bless' and game_id is null;

update spells set grants = '[
  {"target":"attack","mode":"add","dice":"-1d4","source":"Bane"},
  {"target":"save","mode":"add","dice":"-1d4","source":"Bane"}
]'::jsonb where key = 'sp_bane' and game_id is null;

update spells set grants = '[
  {"target":"ac","mode":"add","value":2,"source":"Shield of Faith"}
]'::jsonb where key = 'sp_shieldoffaith' and game_id is null;
