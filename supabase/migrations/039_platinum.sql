-- =====================================================================
-- 039_platinum.sql
-- odyssey1e — the fourth coin, and the one that is not coming
-- =====================================================================
--
-- 032 seeded gold, silver and copper as a DUMMY so a coin purse had
-- something to accept and everything else to refuse. They turned out to
-- be the right shape rather than a placeholder - a coin is an item with
-- the `coin` content tag, which is exactly what HOPPER's `is_coin()`
-- has always tested - so the only thing missing was the top of the
-- ladder.
--
-- NO ELECTRUM, and this is where that is written down. 5e mints it at
-- fifty copper and `items.denom` still permits the value, because a
-- price could be written that way and a conversion that refuses real
-- data is worse than one nobody uses. But there is no `coin_ep` row and
-- there will not be one.
--
-- There is a second, better reason than taste. `make_change` is greedy,
-- largest-denomination-first, and greedy change is only optimal when
-- each denomination divides the next: 1, 10, 100, 1000 does, and
-- inserting 50 between 10 and 100 breaks it. With electrum in the till
-- a shopkeeper could hold enough to make change and fail to find it.
--
-- WEIGHT IS THE BOOK'S. Fifty coins to the pound, so 0.02 each - the
-- same figure 032 gave the other three, and the reason a thousand gold
-- pieces weigh twenty pounds and an adventurer notices.
--
-- VALUE IS NOT STORED. A platinum piece is priced 1 pp like any other
-- item, and `currency::to_cp` makes 1000 copper of those two columns.
-- A coin being worth itself is not a fact worth a column.
-- =====================================================================

insert into public.items
  (key, game_id, name, kind, slots, content_tags, price, denom, weight)
values
  ('coin_pp', null, 'Platinum Piece', 'loot', 0.2, array['coin']::text[], 1, 'pp', 0.02);

comment on column public.items.denom is
  'The coin a price is written in: cp sp ep gp pp. ELECTRUM IS CONVERTIBLE BUT NOT MINTED - see 039. currency::cp_per knows it at fifty copper so a price written that way still resolves, and no coin_ep row exists.';
