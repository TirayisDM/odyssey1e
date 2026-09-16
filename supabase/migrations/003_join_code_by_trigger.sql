-- =====================================================================
-- 003_join_code_by_trigger.sql
-- odyssey1e — fix a bug introduced by 002
-- =====================================================================
--
-- SYMPTOM: creating a game failed with
--   "permission denied for function gen_join_code (403)"
--
-- CAUSE: games.join_code defaulted to gen_join_code(). A COLUMN DEFAULT
-- is evaluated as the INSERTING role. 002 revoked EXECUTE on that
-- function from everyone to clean up the exposed RPC surface — and in
-- doing so made inserting a game impossible.
--
-- WHY NOT JUST GRANT IT BACK: that fixes the insert and simultaneously
-- re-exposes the function at /rest/v1/rpc/gen_join_code, undoing the
-- point of 002. Postgres does NOT check EXECUTE on a trigger function
-- against the triggering user — the system invokes it — so moving the
-- generation into a BEFORE INSERT trigger satisfies both.
--
-- The alphabet logic is INLINED rather than calling gen_join_code(),
-- because an ordinary (non-definer) trigger function runs as the invoker
-- and would hit exactly the same wall one level down.
--
-- THE GENERAL LESSON: anything reachable from a column default, a CHECK
-- constraint, or a generated column runs as the CALLER and needs
-- EXECUTE. Only trigger bodies are exempt. Worth checking against every
-- future "revoke all on function".
-- =====================================================================

alter table public.games alter column join_code drop default;
drop function if exists public.gen_join_code();

create function public.set_join_code()
returns trigger
language plpgsql
set search_path = ''
as $$
declare
  alphabet constant text := 'ABCDEFGHJKMNPQRSTUVWXYZ23456789';
  candidate text;
  i int;
  tries int := 0;
begin
  if new.join_code is not null and btrim(new.join_code) <> '' then
    return new;                      -- caller supplied one; respect it
  end if;

  loop
    candidate := '';
    for i in 1..8 loop
      candidate := candidate || substr(alphabet, 1 + floor(random() * length(alphabet))::int, 1);
    end loop;

    exit when not exists (select 1 from public.games g where g.join_code = candidate);

    tries := tries + 1;
    if tries > 10 then
      raise exception 'could not find a free join code after 10 tries';
    end if;
  end loop;

  new.join_code := candidate;
  return new;
end;
$$;

comment on function public.set_join_code() is
  'Generates games.join_code on insert. A trigger rather than a column default: defaults run as the inserting role and would need EXECUTE granted, which would also expose the function as an RPC endpoint. Alphabet omits 0/O/1/I/L — codes get read aloud across a table.';

create trigger games_set_join_code
  before insert on public.games
  for each row execute function public.set_join_code();

revoke all on function public.set_join_code() from public, anon, authenticated;
