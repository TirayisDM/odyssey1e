-- 086. PUT THE BACKFILL SOMEWHERE POSSIBLE.
--
-- 084's backfill said "armour on the body, a shield in the left hand,
-- everything else in the right". It called the right hand a guess and
-- the only one available, which was true about the SIDE and wrong about
-- the rest: `equipped` was true on seven things at once, so seven
-- things landed in one hand.
--
-- That is not a guess, it is an impossible state. `slots::check` would
-- refuse it, which means the first thing a player tried to move would
-- fail with an error about a hand they never filled on purpose.
--
-- THE REAL SHAPE OF THE OLD DATA is that `equipped` meant "in use" for
-- a weapon and "on me" for everything else - one boolean doing two
-- jobs, which is exactly why 084 split it. So this places each thing by
-- WHAT IT IS, the same way slots::admits does, and keeps the hands for
-- the one thing a hand is for.
--
-- ONE WEAPON IN HAND, the heaviest, because that is the one somebody
-- was most likely actually holding and because a rule has to be
-- deterministic to be re-runnable. PARTITIONED BY HOLDER, so a
-- character and the goblin they are fighting each keep their own.

update objects set slot = null
 where slot is not null
   and slot not in ('body', 'left_hand');

with ranked as (
  select o.id,
         row_number() over (
           partition by o.holder_id
           order by i.weight desc nulls last, i.name
         ) as weapon_rank
    from objects o
    join items i on i.key = o.item_key
   where o.slot is null
     and i.kind = 'weapon'
)
update objects o set slot = 'right_hand'
  from ranked r
 where o.id = r.id and r.weapon_rank = 1;

update objects o set slot = i.worn_slot
  from items i
 where i.key = o.item_key
   and o.slot is null
   and i.worn_slot in ('amulet', 'head');

update objects o set slot = 'backpack'
  from items i
 where i.key = o.item_key and o.slot is null
   and i.kind = 'container' and i.key = 'backpack';

update objects o set slot = 'chest'
  from items i
 where i.key = o.item_key and o.slot is null
   and i.kind = 'container' and i.key = 'chest';

-- THE BELT, and only as much of it as there is room for. Small
-- containers first - a purse and a sheath are what a belt is for - then
-- whatever small thing is left, up to the six slots::HIP_PLACES allows.
with belt as (
  select o.id,
         row_number() over (
           partition by o.holder_id
           order by case when i.kind = 'container' then 0 else 1 end,
                    i.name
         ) as place
    from objects o
    join items i on i.key = o.item_key
   where o.slot is null
     and o.holder_id is not null
     and i.size in ('tiny', 'sm')
     and i.kind in ('container', 'weapon')
)
update objects o set slot = 'hip'
  from belt b
 where o.id = b.id and b.place <= 6;
