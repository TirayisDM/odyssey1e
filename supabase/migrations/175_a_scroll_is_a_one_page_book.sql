-- 175. A SCROLL IS A ONE-PAGE BOOK.
--
-- Dave's words, and they settle the shape. 172 already made a scroll
-- storable - it is the same `scribed_spells` table with one row - but
-- nothing said how much a scroll holds, so nothing could write one.
--
-- ---------------------------------------------------------------------
-- ONE PAGE, AND A SPELL ON A SCROLL TAKES ONE
-- ---------------------------------------------------------------------
--
-- `spell_levels = 1`, so exactly one spell fits and a second is
-- refused by the same `scribe::fits` a full book is refused by.
--
-- AND THE PAGE COST IS ONE WHATEVER THE LEVEL, which is the part that
-- does not follow from the column. In a book a 3rd-level spell takes
-- three levels of room; on a scroll it takes the scroll. A rule that
-- charged three pages against a one-page scroll would make every scroll
-- a cantrip scroll, which is not what a scroll is for. That is
-- `scribe::pages_on`, and `scribe::fits_on` is the capacity question
-- asked of one particular thing.
--
-- ---------------------------------------------------------------------
-- A TAG, BECAUSE `spell_levels` NOW MEANS TWO THINGS
-- ---------------------------------------------------------------------
--
-- `in_books` finds what a wizard may prepare from by looking for items
-- with a `spell_levels`, and giving a scroll one would have quietly put
-- scrolls in that set - a wizard preparing straight off a scroll, which
-- is the one thing 5e is firm about not allowing. You copy a scroll
-- into the book and prepare from the book.
--
-- SO THE TAG IS THE DISCRIMINATOR and `content_tags` is where this
-- schema already keeps that kind of fact - 126 put `natural` on a
-- wolf's teeth for the same reason, and said it was a hook for whoever
-- needed it. This is that.
--
-- NOT A COLUMN. `is_scroll boolean` would be a third thing to keep in
-- step with `spell_levels` and the tag, and two of the three would
-- drift. One tag, read in one place.
--
-- ---------------------------------------------------------------------
-- WHAT IT COSTS, AND WHAT A SCROLL IS FOR
-- ---------------------------------------------------------------------
--
-- WRITING ONE IS TWICE WRITING IT DOWN - `scribe::to_scroll`. Copying
-- into a book is transcription: the spell is understood and the book is
-- a reference. A scroll carries the whole working on its own, for
-- somebody who may not understand it, which is the harder job.
--
-- COPYING ONE INTO A BOOK COSTS WHAT COPYING COSTS, and destroys the
-- scroll - `copy_from_scroll`. 172's cascade does the second half: the
-- object goes and its one row goes with it.

update public.items
   set spell_levels = 1,
       content_tags = array['scribing', 'scroll'],
       description = 'A sheet of prepared vellum, ruled and sized. One spell fits on it, '
                     'whatever its level, and writing one takes twice what the same spell '
                     'costs in a book - a scroll has to carry the whole working on its own, '
                     'for somebody who may not understand it.'
 where game_id is null and key = 'scroll_blank';
