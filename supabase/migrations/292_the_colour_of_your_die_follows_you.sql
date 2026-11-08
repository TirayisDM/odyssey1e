/* ===================================================================
   292. THE COLOUR OF YOUR DIE FOLLOWS YOU
   ===================================================================

   Dave, on 290's die: *"next add a color selector for Dice Color:
   Red, Yellow, White. And number color of: White, Black. This should
   get saved for user in a table."*

   IN A TABLE, AND HE SAID SO. This application already remembers two
   preferences and both are in `localStorage`, deliberately: whether
   the feed is open and which of its tabs you left showing are facts
   about THIS MACHINE. The colour of your die is a fact about YOU, and
   a player who sits down at the laptop should find their own die
   waiting rather than a white one. That is the distinction between
   the two stores, and it is now worth stating because this is the
   first preference to land on the other side of it.

   NOT TWO COLUMNS ON `profiles`, and the reason was checked rather
   than assumed. `profiles`'s own policies are still 001's and they
   read `id = auth.uid()`. 197 split a PERSON from a SIGN-IN and gave
   us `current_profile()` to resolve a second identity back to the one
   human being - so a preference hung on `profiles` would save
   correctly for everybody except the person 197 exists for. Keyed on
   `current_profile()`, this table is right for both, and identity's
   policies are left alone to be looked at on their own terms.

   THE COLOURS ARE NAMED HERE, ONCE. Three for the die, two for the
   numbers, as CHECK constraints rather than a list in JavaScript that
   agrees with this one until the day somebody adds green to one of
   them. That is 288's lesson about a rule written twice, and the
   screen reads these same names back off the row.

   WHAT A NAME IS AND IS NOT. `white` is a choice, not a hex value.
   What white looks like on a face, on its lit face, and against black
   numbers is the stylesheet's business - which is 283's reasoning
   about `ring` being a style id, and it means the palette can be
   retuned without a migration.

   THE DEFAULTS ARE A PLAIN DIE: white faces, black numbers. 290's
   die was neither - it was the stylesheet's grey, which is not one of
   the three colours Dave named - so something had to be chosen, and
   the plain one is it. The stylesheet's base die matches these two,
   so a die drawn before the preference has loaded is the die the
   preference will almost always ask for.

   =================================================================== */

create table public.dice_prefs (
  profile_id  uuid primary key
              references public.profiles(id) on delete cascade
              default public.current_profile(),
  die_colour  text not null default 'white'
              check (die_colour = any (array['red', 'yellow', 'white'])),
  pip_colour  text not null default 'black'
              check (pip_colour = any (array['white', 'black'])),
  updated_at  timestamptz not null default now()
);

comment on table public.dice_prefs is
  '292. How one person wants their die to look. One row per PERSON, not per sign-in and not per device - 197 for why that distinction matters, and the two localStorage preferences in main.js for what lives on the other side of it.';
comment on column public.dice_prefs.profile_id is
  'Whose die. Defaults to current_profile() so an insert never has to name it, and the policies will not accept anybody else''s.';
comment on column public.dice_prefs.die_colour is
  'red, yellow or white. A NAME AND NOT A COLOUR VALUE: what white looks like - unlit, lit, crit, fumble - is the stylesheet''s to decide, so the palette can be retuned without a migration.';
comment on column public.dice_prefs.pip_colour is
  'white or black, for the numbers on the faces. Two rather than three because the job is contrast against the die, not decoration.';

alter table public.dice_prefs enable row level security;

-- YOUR OWN ROW AND NOTHING ELSE. There is no DM exception and no
-- fellow-member read: unlike a display name (198), nobody else at the
-- table has any business knowing what colour your die is, and the roll
-- that matters to them is already in the feed.
create policy "dice_prefs: read own"
  on public.dice_prefs for select to authenticated
  using (profile_id = public.current_profile());

create policy "dice_prefs: insert own"
  on public.dice_prefs for insert to authenticated
  with check (profile_id = public.current_profile());

create policy "dice_prefs: update own"
  on public.dice_prefs for update to authenticated
  using (profile_id = public.current_profile())
  with check (profile_id = public.current_profile());

-- No delete policy, and that is the whole statement: there is no such
-- thing as un-choosing a colour. Setting it back to white is an update.

-- 195's check. It passes because this adds no function at all - only a
-- table whose three policies resolve the caller through
-- `current_profile()` rather than taking anyone's word for it.
do $gate$
declare n int;
begin
  select count(*) into n from public.security_doors();
  if n > 0 then
    raise exception '292: the wall has % hole(s)', n;
  end if;
end
$gate$;
