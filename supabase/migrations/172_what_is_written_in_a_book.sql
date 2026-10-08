-- 172. WHAT IS WRITTEN IN A BOOK.
--
-- The decision 170 and 171 were waiting on, taken: a book's contents
-- belong to the BOOK, not to the character holding it.
--
-- 150 made a wizard's book `character_prayers` rows in state 'book',
-- which is where `may_reach` reads from and is tested. Those rows
-- belong to a CHARACTER, and three things fall out of that which Dave's
-- four books make impossible to live with:
--
--   * a spellbook FOUND in a dungeon cannot have anything written in
--     it - there is no character to hang the rows on
--   * a SECOND book means nothing, because no row says which book a
--     spell is in, so 171's 10/20/30/50 levels have nothing to count
--   * a SCROLL cannot exist at all - it is an unowned object with a
--     spell on it, the same problem from the other end
--
-- ---------------------------------------------------------------------
-- WRITING IS NOT GEAR, WHICH IS WHY THIS IS NOT AN `objects` ROW
-- ---------------------------------------------------------------------
--
-- 027 wrote `spell_book accepts {spell}` and meant a spell to be an
-- object inside a container. The container machinery is real and 030's
-- entities would carry it - but the operations are wrong. An object can
-- be dropped on the floor, equipped, stacked, split and sold on its
-- own. A page can be written, erased and copied, and that is the whole
-- list. Modelling writing as gear invites every operation that makes no
-- sense and then needs a guard against each one.
--
-- So: one narrow table, and `accepts = {spell}` stays what it has
-- always been - an intention from 027 that nothing reads.
--
-- ---------------------------------------------------------------------
-- WHAT THE SHAPE SAYS
-- ---------------------------------------------------------------------
--
-- KEYED TO THE OBJECT, so a book carries its contents wherever it goes:
-- stolen, lent, left in a cart, taken off a body. 030 already says an
-- object outlives its holder and this inherits that for free.
--
-- ON DELETE CASCADE, because burning the book burns what is in it. That
-- is the rule a player expects and the only one that does not leave
-- rows pointing at nothing.
--
-- PRIMARY KEY (object_id, spell_key) - a spell is written once in a
-- book. Writing it twice is not two copies, it is the same page.
--
-- NO `game_id`. The object has one and RLS reads it through the object,
-- which is also what keeps a book's contents exactly as reachable as
-- the book: if you can see the book you can read it, and if you could
-- move it you can write in it.
--
-- A SCROLL IS THE SAME TABLE WITH ONE ROW. Nothing here distinguishes
-- them - the difference is the item: a book has `spell_levels` and a
-- scroll does not, and `scribe.rs` decides what that means.
--
-- ---------------------------------------------------------------------
-- WHAT THIS TABLE DOES NOT ENFORCE
-- ---------------------------------------------------------------------
--
-- CAPACITY, AND THAT IS ON PURPOSE. `scribe::fits` owns how much room a
-- spell takes - one level per level, floored at one for a cantrip - and
-- it is tested. A trigger here would be the same rule in a second
-- place, which is the fault this schema keeps paying for. Rust refuses
-- before it writes.
--
-- THAT ONLY A BOOK IS WRITTEN IN, for the same reason. Writing a spell
-- into a sword is nonsense, and it is `scribe.rs`'s nonsense to refuse.

create table if not exists public.scribed_spells (
  object_id  uuid not null references public.objects(id) on delete cascade,
  spell_key  text not null,
  scribed_at timestamptz not null default now(),
  primary key (object_id, spell_key)
);

comment on table public.scribed_spells is
  'What is written in one book or scroll. Keyed to the OBJECT so the '
  'writing travels with the thing - found, stolen, lent or burnt. '
  'Capacity and who may write are rules in scribe.rs, not here. See 172.';

-- Reverse lookup: every book in the world that holds Fireball. Wanted
-- the first time a DM asks where a spell can be found.
create index if not exists scribed_spells_by_spell
  on public.scribed_spells (spell_key);

alter table public.scribed_spells enable row level security;

-- EXACTLY AS REACHABLE AS THE BOOK. These mirror `objects` rather than
-- inventing a second rule: a member of the game reads it, and the DM,
-- the holder's owner, or anybody for an unheld or location-held object
-- writes it. `holder_character` and `holder_is_a_location` are 031's
-- helpers and are the same ones the objects policies call.
drop policy if exists scribed_spells_read on public.scribed_spells;
create policy scribed_spells_read on public.scribed_spells
  for select to authenticated
  using (exists (
    select 1 from public.objects o
     where o.id = scribed_spells.object_id
       and public.is_game_member(o.game_id)));

drop policy if exists scribed_spells_write on public.scribed_spells;
create policy scribed_spells_write on public.scribed_spells
  for insert to authenticated
  with check (exists (
    select 1 from public.objects o
     where o.id = scribed_spells.object_id
       and (public.is_game_dm(o.game_id)
            or exists (select 1 from public.characters c
                        where c.id = public.holder_character(o.holder_id)
                          and c.owner_uid = auth.uid())
            or ((o.holder_id is null or public.holder_is_a_location(o.holder_id))
                and public.is_game_member(o.game_id)))));

drop policy if exists scribed_spells_erase on public.scribed_spells;
create policy scribed_spells_erase on public.scribed_spells
  for delete to authenticated
  using (exists (
    select 1 from public.objects o
     where o.id = scribed_spells.object_id
       and (public.is_game_dm(o.game_id)
            or exists (select 1 from public.characters c
                        where c.id = public.holder_character(o.holder_id)
                          and c.owner_uid = auth.uid())
            or ((o.holder_id is null or public.holder_is_a_location(o.holder_id))
                and public.is_game_member(o.game_id)))));
