-- 135. TWO STATBLOCKS THAT WERE NEVER CREATURES.
--
-- `Goblin Fighter` (key 0000A1) and `Litmor` (key 100003) came off the
-- AppSheet port in 006 and are the last two rows in `npcs` that nobody
-- designed. Both have the spreadsheet's own key shape - six characters
-- of base-36 - where every creature 127 and 130 added is named after
-- itself: `goblin`, `wolf`, `shrieker`.
--
-- THEY HAVE NO KIT AT ALL, which is the fault worth naming. 134
-- statblocks carry 223 rows of equipment between them and these two
-- carry none, equipped or otherwise, so `attack::resolve` finds nothing
-- to swing and a DM who enrols one gets a creature that cannot act.
-- They are not weak; they are unfinished. `Litmor` is not even a kind
-- of thing - it was one character's name.
--
-- WHY A DELETE RATHER THAN A FIX. The alternative is to invent a goblin
-- and a Litmor, and the bestiary already HAS a goblin, designed with
-- its scimitar, its bow and its hide. Giving these two a weapon would
-- leave a second goblin in the catalogue that differs from the real one
-- in ways nobody chose.
--
-- NOTHING POINTS AT THEM, checked before writing this rather than
-- hoped: no row in `characters` names either key (`characters.npc_key`
-- is a loose text reference and not a constraint, so the check had to
-- be a query), no row in `npc_items`, and no foreign key in this schema
-- references `npcs` at all. Deleting them takes nothing with it.
--
-- SAY WHAT KIND OF ROW YOU MEAN - 132's lesson, and this is the first
-- statement written after learning it. `game_id is null` below is
-- scoped by two literal keys as well, so it means these two rows and
-- cannot mean anything else.

delete from npcs
 where game_id is null
   and key in ('0000A1', '100003');
