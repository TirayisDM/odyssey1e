-- =====================================================================
-- 038_one_depth_cap.sql
-- odyssey1e — three walks up the same chain, three different limits
-- =====================================================================
--
-- `holder_character` walks up from a holder to whoever ultimately has
-- the thing, and 031 capped it at eight because odyssey-engine defaults
-- container nesting to six and eight was six plus room.
--
-- Two later walks of the same chain chose thirty-two: `no_location_cycles`
-- in 033, and `holders::root_of` in Rust. Nothing reconciled them.
--
-- WHY THE SMALLEST ONE IS THE DANGEROUS ONE. `holder_character` is not
-- a display function - every policy on `objects` calls it. Running out
-- of guard returns NULL, NULL means "nobody owns this", and the owner
-- branch of the policy then does not match. So a chain deeper than
-- eight does not error: the object simply stops being yours. A read
-- hides it and a delete matches nothing, which is the same silent
-- failure 037 has just finished cleaning up after.
--
-- Eight is not reachable today with a purse in a backpack. It becomes
-- reachable the moment a cart holds chests that hold crates, and it
-- would present as "my sword disappeared".
--
-- THIRTY-TWO, to match the other two. The number is not a rule about
-- how deep containers may go - `no_container_cycles` refuses a loop
-- outright, which is the actual protection. This is only the belt for
-- the braces, and three different belts is worse than one loose one.
-- =====================================================================

create or replace function public.holder_character(p_holder uuid)
returns uuid
language plpgsql
stable
security definer
set search_path = ''
as $$
declare
  cur   uuid := p_holder;
  cid   uuid;
  guard int  := 0;
begin
  while cur is not null and guard < 32 loop
    select id into cid from public.characters where entity_id = cur;
    if cid is not null then
      return cid;
    end if;
    select o.holder_id into cur from public.objects o where o.entity_id = cur;
    guard := guard + 1;
  end loop;
  return null;
end;
$$;

comment on function public.holder_character(uuid) is
  'The character who ultimately has whatever is in this holder, walking up through containers. A purse inside a backpack carried by Rodnar is Rodnar''s. Capped at 32 since 038, matching no_location_cycles and holders::root_of - running out of guard returns NULL, and because every objects policy calls this, NULL reads as "not yours" rather than as an error.';
