-- 094. SOMETHING THAT IS TRUE FOR A WHILE.
--
-- The last piece 092's clock was built for. "Inspires compatriots for 1
-- hour, max does not stack" is two rules - a duration and a stacking
-- rule - and before the clock there was nowhere to measure the first
-- and nowhere to enforce the second.
--
-- AN EFFECT IS A ROW WITH A DEADLINE, in `games.tick`. Expiry is one
-- integer comparison, so an effect behaves identically in a fight and
-- on the road and nothing has to sweep a table to notice an hour has
-- gone by.
--
-- NOTHING DELETES AN EXPIRED EFFECT. It is no longer active and the row
-- stays as a record of what was true at the time - the same contract as
-- a roll. A DM asking "what was on him when he fell" has an answer.
-- `ended_at` is for an effect cut short, which is a different fact from
-- one that simply ran out.
--
-- THE STACKING RULE TRAVELS WITH THE EFFECT rather than living in the
-- engine, because which one applies is a property of the thing: a
-- bard's song replaces, a poison might stack, a blessing might refuse a
-- second. effects.rs holds the four behaviours and this column says
-- which.

create table if not exists effects (
  id            uuid primary key default gen_random_uuid(),
  game_id       uuid not null references games(id) on delete cascade,
  character_id  uuid not null references characters(id) on delete cascade,
  key           text not null,
  name          text not null,
  magnitude     integer,
  -- WHO PUT IT THERE, and with what. Both nullable: a DM can apply
  -- something out of nowhere, which is most of what a DM does.
  source_character_id uuid references characters(id) on delete set null,
  source_feature      text,
  started_at    bigint not null,
  expires_at    bigint,
  ended_at      bigint,
  stacks        text not null default 'replace',
  note          text,
  created_at    timestamptz not null default now(),
  constraint effects_stacks_check
    check (stacks in ('replace', 'highest', 'stack', 'refuse')),
  constraint effects_ends_after_it_starts
    check (expires_at is null or expires_at > started_at)
);

comment on table effects is
  'Something that is true for a while. Expiry is a games.tick comparison, so an effect reads the same in a fight and on the road. Rows are NEVER deleted when they run out - an expired effect is a record of what was true, on the same contract as a roll. The arithmetic and the stacking rules are effects.rs.';
comment on column effects.key is
  'What KIND this is - inspired, bless, poisoned. Two effects with the same key are the same thing, and stacking is decided across that and nothing else: a blessing and a bard''s song never interact, because deciding they are "the same bonus" is a rule about those two things and not one 5e makes.';
comment on column effects.magnitude is
  'How strong, where strength means anything - the die size of a bard''s inspiration, the size of a bonus. NULL where the effect is simply on or off. Read by the `highest` stacking rule and by nothing else.';
comment on column effects.expires_at is
  'The games.tick it stops being true at. NULL runs until something ends it, which is not the same as forever - a curse needs a DM, not arithmetic.';
comment on column effects.ended_at is
  'Set when an effect was cut short rather than running out. A different fact from expiry and worth keeping apart: dispelling a blessing and watching it lapse are not the same event.';
comment on column effects.stacks is
  'What happens when a second of the same key arrives: replace, highest, stack, refuse. It travels with the effect because which one applies is a property of the thing. `highest` is the subtle one - a weaker version of what is already running is NOTHING rather than an error, so a bard who sings badly cannot undo their own good song.';

create index if not exists effects_on_character on effects (character_id, key);
create index if not exists effects_in_game on effects (game_id);

alter table effects enable row level security;

drop policy if exists "effects: read in game" on effects;
create policy "effects: read in game" on effects
  for select using (is_game_member(game_id));

-- ANYBODY IN THE GAME MAY APPLY ONE, because a bard inspiring the party
-- is a player doing something to somebody else and the alternative is
-- every buff going through the DM. Ending one is narrower: the DM, or
-- whoever it is on.
drop policy if exists "effects: members apply" on effects;
create policy "effects: members apply" on effects
  for insert with check (is_game_member(game_id));

drop policy if exists "effects: dm or bearer ends" on effects;
create policy "effects: dm or bearer ends" on effects
  for update using (
    is_game_dm(game_id)
    or exists (select 1 from characters c
                where c.id = effects.character_id and c.owner_uid = auth.uid()))
  with check (
    is_game_dm(game_id)
    or exists (select 1 from characters c
                where c.id = effects.character_id and c.owner_uid = auth.uid()));

-- NO DELETE POLICY AT ALL, deliberately. An effect that ran out is a
-- record and an effect cut short is `ended_at`; neither is a reason to
-- remove the row, and RLS denying what nothing asks for is the cheapest
-- way to keep it that way.
drop policy if exists "effects: nobody deletes" on effects;
