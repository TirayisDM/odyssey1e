/* ===================================================================
   283. A FACE FOR A CHARACTER
   ===================================================================

   Dave: *"I want to establish a top section that shows the character
   icon - round with a border of 10 px so we can add customized
   borders"*, and then: *"The edit will allow image control panel -
   asking for image source, title, rank (in display weight), default
   status, ring style (sub menu to choose later for now do 1-10
   colors). After choosing a save should write to memory and the menu
   closes to just display image with border."*

   TITLE, RANK AND DEFAULT ARE WHY THIS IS A TABLE. A single image has
   nothing to be titled apart from, nothing to be ranked among, and
   nothing to be the default instead of. Those three fields say a
   character has SEVERAL pictures, so this is a child table of
   `characters` - the shape abilities, skills, items and dice all take
   - rather than a column on it.

   `portrait_url` GOES. 001 gave `characters` a `portrait_url text`
   and nothing has ever read or written it: no select list names it,
   no command patches it, and all 26 rows in the live project are
   null. Leaving it beside a table that actually holds the pictures
   would install the two-places-disagree fault 280 had just finished
   taking out of the character lists, this time in the schema. 115 is
   that an applied migration is a record and not a draft, which is why
   this drops the column rather than editing 001.

   THE RING IS A NUMBER, NOT A COLOUR. `ring` stores 1-10: the style
   somebody chose. What each one looks like is the stylesheet's to
   know. The submenu Dave deferred is where a style becomes a gradient
   or a worn-bronze texture, and that has to be possible without a
   migration - which it is, if what travels is the choice.

   ONE DEFAULT, ENFORCED BY THE DATABASE. A partial unique index means
   a second default is refused rather than relied upon not to happen.
   `save_character_image` clears the old one in the same call, so the
   refusal is a backstop nobody should ever meet.

   THE URL IS CHECKED IN BOTH PLACES, here and in Rust. The webview
   runs with `"csp": null`, so an `<img src>` is handed whatever is
   stored - and `javascript:` must not get as far as the column. The
   CHECK is the one that cannot be forgotten.

   =================================================================== */

create table public.character_images (
  id            uuid primary key default gen_random_uuid(),
  character_id  uuid not null references public.characters(id) on delete cascade,
  url           text not null check (url ~* '^https?://'),
  title         text,
  rank          integer not null default 0,
  is_default    boolean not null default false,
  ring          smallint not null default 1 check (ring between 1 and 10),
  created_at    timestamptz not null default now()
);

comment on table public.character_images is
  'The pictures of one character. 283. Several per character, because title, rank and default status only mean anything when there is more than one.';
comment on column public.character_images.url is
  'Where the picture is. http or https only, checked here as well as in Rust - the webview has no CSP and is handed this value directly.';
comment on column public.character_images.title is
  'What this picture is of - "before the curse", "in plate". Optional; an untitled picture is a normal picture.';
comment on column public.character_images.rank is
  'Display weight. Higher sorts earlier. Ties break on created_at, so an unranked gallery is simply the order they were added.';
comment on column public.character_images.is_default is
  'The one the sheet shows. At most one per character - see character_images_one_default.';
comment on column public.character_images.ring is
  'Which ring style surrounds it, 1-10. A STYLE ID AND NOT A COLOUR: the stylesheet decides what 7 looks like, so a ring can become a gradient or a texture without a migration.';

create unique index character_images_one_default
  on public.character_images(character_id) where is_default;

create index character_images_by_character
  on public.character_images(character_id);

alter table public.character_images enable row level security;

-- 005's pattern, unchanged: visibility follows the parent character
-- rather than repeating the membership test, and a character you
-- cannot see yields no image rows.
create policy "character_images: read with character"
  on public.character_images for select
  using (exists (select 1 from public.characters c
                 where c.id = character_id and public.is_game_member(c.game_id)));

create policy "character_images: owner or dm writes"
  on public.character_images for insert
  with check (exists (select 1 from public.characters c
                      where c.id = character_id
                        and (c.owner_uid = auth.uid() or public.is_game_dm(c.game_id))));

create policy "character_images: owner or dm updates"
  on public.character_images for update
  using (exists (select 1 from public.characters c
                 where c.id = character_id
                   and (c.owner_uid = auth.uid() or public.is_game_dm(c.game_id))))
  with check (exists (select 1 from public.characters c
                      where c.id = character_id
                        and (c.owner_uid = auth.uid() or public.is_game_dm(c.game_id))));

create policy "character_images: owner or dm deletes"
  on public.character_images for delete
  using (exists (select 1 from public.characters c
                 where c.id = character_id
                   and (c.owner_uid = auth.uid() or public.is_game_dm(c.game_id))));


-- =====================================================================
-- AND THE COLUMN THAT NEVER HELD A FACE
-- =====================================================================

alter table public.characters drop column portrait_url;
