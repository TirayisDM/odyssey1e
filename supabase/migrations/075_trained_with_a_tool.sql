-- 075. TRAINED WITH A TOOL.
--
-- 5e makes a musical instrument a TOOL PROFICIENCY, and a bard gets
-- three of their choice. The engine had two proficiency lists -
-- `weapon_profs` and `armor_profs`, both text arrays off the Foundry
-- export in 008 - and no third.
--
-- THE SAME SHAPE AS THE OTHER TWO, on purpose. `equipment::is_proficient`
-- already takes the first two and ends in a catch-all whose comment
-- said "nothing else grants or needs proficiency". That is now false,
-- and a third slice through the same function is a smaller change than
-- a parallel mechanism beside it. Values are `items.key` - `lute`,
-- `drum` - which is how `weapon_profs` already names a base item.
--
-- NOT A SEPARATE TABLE. `character_skills` is a table because a skill
-- carries a DEGREE - 0.5 for half, 2.0 for expertise. A tool is a
-- yes or a no, which is what an array is for, and 008 made that call
-- twice already.
--
-- EMPTY FOR EVERY CHARACTER THAT EXISTS, and that is honest rather
-- than a gap to backfill: nobody has been trained with an instrument
-- because until 074 there were no instruments to be trained with.

alter table characters
  add column if not exists tool_profs text[] not null default '{}';

comment on column characters.tool_profs is
  'Tools this character is trained with, by items.key - an instrument is the only kind so far. The third list beside weapon_profs and armor_profs and read through the same equipment::is_proficient, because a tool proficiency is the same yes-or-no question about a different kind of thing. A DEGREE would need a table: character_skills is one because 0.5 and 2.0 are real answers for a skill and not for a tool.';

-- WHICH TOOLS A CLASS HANDS OUT, and how many of them.
--
-- Two columns rather than one, because 5e's bard does not get three
-- NAMED instruments - it gets three OF THEIR CHOICE. `tool_choices` is
-- how many, `tool_grants` is any that come without a choice. A class
-- that grants outright (a smith's tools, say) fills the second and
-- leaves the first at zero.
--
-- NEITHER IS ENFORCED YET. Creation does not offer the choice and
-- nothing counts what has been chosen against the allowance - the
-- columns state what a class is OWED so that the screen which asks has
-- something to ask from. `skill_choices` has sat in exactly this state
-- since 055 for the same reason.
alter table classes
  add column if not exists tool_choices integer not null default 0,
  add column if not exists tool_grants text[] not null default '{}';

comment on column classes.tool_choices is
  'How many tool proficiencies this class picks freely. Bard is 3 - "three musical instruments of your choice". Not enforced: nothing counts chosen against owed, exactly as skill_choices has been since 055.';
comment on column classes.tool_grants is
  'Tools this class grants outright, by items.key, with no choice involved. Empty for the bard, whose three are all chosen.';

update classes set tool_choices = 3 where key = 'bard' and game_id is null;
