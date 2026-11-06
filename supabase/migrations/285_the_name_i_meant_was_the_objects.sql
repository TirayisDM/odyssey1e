/* ===================================================================
   285. THE NAME I MEANT WAS THE OBJECT'S
   ===================================================================

   Dave, with a screenshot of the first real drop: *"Storage said 400:
   new row violates row-level security policy"*.

   284's four policies on `storage.objects` all read
   `storage.foldername(name)`, meaning the object's key -
   `<character_id>/<hex>.png` - whose first folder is the character id
   the policy then joins on. That is not what Postgres resolved. The
   EXISTS subquery selects `from public.characters c`, and an
   unqualified `name` inside it binds to the INNER relation first. So
   what was stored, and what pg_policies prints back, is:

     (c.id)::text = (storage.foldername(c.name))[1]

   The CHARACTER's name. `storage.foldername('Bryn')` is `{}`, so
   `[1]` is NULL, so the comparison is NULL, so the EXISTS is false -
   for every object, every character and every command. The write was
   refused, and the read would have returned nothing had anything ever
   got written.

   IT COMPILED, IT APPLIED, AND `security_doors()` PASSED, because
   there is nothing malformed about it: `characters` genuinely has a
   `name` column, and a policy that is always false is a policy, not
   an error. The only thing that could have caught it was an insert -
   which is exactly the step 284's own carried note said had not been
   run.

   THE FIX IS ONE QUALIFIED REFERENCE. `objects.name` can only be the
   protected table's column, so the shadowing cannot happen again in
   these four. The same trap is waiting in any future policy whose
   subquery selects from a table that also has the column being
   tested, and the lesson is the general one: inside an EXISTS, QUALIFY
   THE OUTER COLUMN.

   WHAT THIS PROVES ABOUT THE REST OF 284. The error came back from
   the INSERT on `storage.objects`, which means the upload reached
   Storage with its bytes, its content type and a token Storage
   accepted, and got as far as the row. Everything 284 could not
   verify without somebody at the keyboard worked except this.

   115 is why this is a new file rather than an edit to 284: an
   applied migration is a record of what the database was told, not a
   draft of what it should have been told.

   =================================================================== */

drop policy "character images: read with character" on storage.objects;
drop policy "character images: owner or dm writes" on storage.objects;
drop policy "character images: owner or dm updates" on storage.objects;
drop policy "character images: owner or dm deletes" on storage.objects;

create policy "character images: read with character"
  on storage.objects for select
  using (
    bucket_id = 'character-images'
    and exists (
      select 1 from public.characters c
      where c.id::text = (storage.foldername(objects.name))[1]
        and public.is_game_member(c.game_id)
    )
  );

create policy "character images: owner or dm writes"
  on storage.objects for insert
  with check (
    bucket_id = 'character-images'
    and exists (
      select 1 from public.characters c
      where c.id::text = (storage.foldername(objects.name))[1]
        and (c.owner_uid = auth.uid() or public.is_game_dm(c.game_id))
    )
  );

create policy "character images: owner or dm updates"
  on storage.objects for update
  using (
    bucket_id = 'character-images'
    and exists (
      select 1 from public.characters c
      where c.id::text = (storage.foldername(objects.name))[1]
        and (c.owner_uid = auth.uid() or public.is_game_dm(c.game_id))
    )
  )
  with check (
    bucket_id = 'character-images'
    and exists (
      select 1 from public.characters c
      where c.id::text = (storage.foldername(objects.name))[1]
        and (c.owner_uid = auth.uid() or public.is_game_dm(c.game_id))
    )
  );

create policy "character images: owner or dm deletes"
  on storage.objects for delete
  using (
    bucket_id = 'character-images'
    and exists (
      select 1 from public.characters c
      where c.id::text = (storage.foldername(objects.name))[1]
        and (c.owner_uid = auth.uid() or public.is_game_dm(c.game_id))
    )
  );
