/* ===================================================================
   293. AND THE COLUMN THAT SAYS WHEN
   ===================================================================

   292 gave `dice_prefs` an `updated_at` and wired nothing to it, so
   it would have held the moment the row was created and called that
   the moment it was changed. One line short of true.

   `touch_updated_at()` HAS EXISTED SINCE 001 and `characters` has
   been using it since 001 - which is the point: there was no decision
   to make here and no client-side timestamp to send, only a trigger
   to attach that I did not attach. BEFORE UPDATE, exactly as
   `characters_touch_updated_at` does it, so an insert keeps the
   column default and every change after it tells the truth.

   THIS IS A SEPARATE FILE BECAUSE 292 IS ALREADY APPLIED. 115: an
   applied migration is a record of what the database was told, not a
   draft of what it should have been told. The two files are one
   cycle's work and the ledger will show both.

   =================================================================== */

create trigger dice_prefs_touch_updated_at
  before update on public.dice_prefs
  for each row execute function public.touch_updated_at();
