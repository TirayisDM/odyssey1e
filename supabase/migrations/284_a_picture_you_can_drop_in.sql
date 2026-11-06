/* ===================================================================
   284. A PICTURE YOU CAN DROP IN
   ===================================================================

   Dave: *"need to make it so we can paste an image to upload it, as an
   option"*, and then: *"should we just start with drag an image in
   from explorer, and add the paste later or is it same basic
   operation? If it is start it."*

   IT IS THE SAME OPERATION. A drop hands the webview a File from
   `dataTransfer.files` and a paste hands it one from
   `clipboardData.items`; everything after that - the bytes, the size
   cap, the sniff, the upload, the row - is one path. So both arrive
   here together, because building one and withholding the other would
   have been a deliberate omission rather than a smaller cycle.

   283 STORED SOMEBODY ELSE'S ADDRESS. That is still offered and still
   useful, and it has the failure the panel already names: the picture
   disappears when that server does. An upload needs the bytes, and the
   bytes need Supabase Storage - which this project has never touched.
   No bucket, no policy on `storage.objects`, nothing in Rust.

   A PRIVATE BUCKET, WHICH IS THE WHOLE REASON THIS FILE IS LONGER THAN
   IT LOOKS. A public bucket would hand back a plain https URL that
   works forever and fits 283's column unchanged - and it would be the
   one place in this application where a thing is readable without an
   account, a membership or a policy. 203's line is that `amDM()` is an
   offer and RLS is the boundary; a portrait in somebody's private
   campaign is exactly the thing that should not be reachable by link.
   So the bucket is private, `storage.objects` carries the same four
   policies the table does, and the cost is paid on the read:
   `load_images` mints a signed URL per picture, good for a day.

   THE PATH IS THE POLICY. An object is stored at
   `<character_id>/<uuid>.<ext>`, so `storage.foldername(name)[1]` IS
   the character id and the policies below can join to `characters`
   exactly as the table's do. Nothing else about the name is load
   bearing - the uuid is there so two files called `portrait.png` do
   not collide, and the extension is the SNIFFED one rather than
   whatever the dropped file was called.

   ONE SOURCE PER ROW, NOT TWO. `url` and `storage_path` are the two
   kinds of the same picture, and a row has exactly one of them. A
   CHECK says so, because the alternative is a row with both where
   nothing decides which wins. 283's `url ~* '^https?://'` moves inside
   that CHECK, since the column is now nullable and the old constraint
   would have refused every upload.

   =================================================================== */

alter table public.character_images alter column url drop not null;
alter table public.character_images drop constraint character_images_url_check;

alter table public.character_images add column storage_path text;

comment on column public.character_images.storage_path is
  'Where the uploaded file lives in the character-images bucket, as <character_id>/<uuid>.<ext>. NULL for a picture given as an address - see character_images_one_source.';
comment on column public.character_images.url is
  'Somebody else''s address for this picture. NULL for an upload, which carries storage_path instead.';

alter table public.character_images add constraint character_images_one_source
  check (
    (url is not null and storage_path is null and url ~* '^https?://')
    or
    (url is null and storage_path is not null)
  );


-- =====================================================================
-- THE BUCKET
-- =====================================================================

-- `public` false is the point of the whole file. The size limit and the
-- mime list are a second fence behind Rust's: Rust reads the magic
-- bytes and refuses anything that is not one of these four, and this
-- refuses it again if a bug ever lets one past.
insert into storage.buckets (id, name, public, file_size_limit, allowed_mime_types)
values (
  'character-images',
  'character-images',
  false,
  2097152,
  array['image/png', 'image/jpeg', 'image/webp', 'image/gif']
);

create policy "character images: read with character"
  on storage.objects for select
  using (
    bucket_id = 'character-images'
    and exists (
      select 1 from public.characters c
      where c.id::text = (storage.foldername(name))[1]
        and public.is_game_member(c.game_id)
    )
  );

create policy "character images: owner or dm writes"
  on storage.objects for insert
  with check (
    bucket_id = 'character-images'
    and exists (
      select 1 from public.characters c
      where c.id::text = (storage.foldername(name))[1]
        and (c.owner_uid = auth.uid() or public.is_game_dm(c.game_id))
    )
  );

create policy "character images: owner or dm updates"
  on storage.objects for update
  using (
    bucket_id = 'character-images'
    and exists (
      select 1 from public.characters c
      where c.id::text = (storage.foldername(name))[1]
        and (c.owner_uid = auth.uid() or public.is_game_dm(c.game_id))
    )
  );

create policy "character images: owner or dm deletes"
  on storage.objects for delete
  using (
    bucket_id = 'character-images'
    and exists (
      select 1 from public.characters c
      where c.id::text = (storage.foldername(name))[1]
        and (c.owner_uid = auth.uid() or public.is_game_dm(c.game_id))
    )
  );
