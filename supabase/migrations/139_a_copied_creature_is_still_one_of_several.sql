-- 139. A COPIED CREATURE IS STILL ONE OF SEVERAL.
--
-- 138 taught `enrol_actor` to copy a template into a fight, and it works
-- - Webbys the giant spider walked into the Tavern, rolled an 18 for
-- initiative and bit Falon for 17. What it did not do is give the actor
-- any PROVENANCE, and that turns out to decide something.
--
-- ---------------------------------------------------------------------
-- 018's NAMING IS KEYED ON npc_key, AND A TEMPLATE HAS NONE
-- ---------------------------------------------------------------------
--
-- `name_actor()` has two branches and no third:
--
--   npc_key set    -> "Goblin 0001", numbered across the whole game,
--                     and the character is renamed to match
--   npc_key null   -> take the character's name verbatim, no ordinal
--
-- A character enrolled as themselves wants the second: Falon is Falon,
-- and there is only ever one of him. 138's copies landed there too,
-- because the statblock key is on the CHARACTER row and not on the
-- actor. So the first Webbys is "Webbys" and so is the second, and so
-- is the third - three actors with one name, indistinguishable in the
-- roster, in the turn order, and in `rolls.character_name`, which
-- snapshots the name at the moment of the roll and cannot be untangled
-- afterwards.
--
-- That is the many-goblins problem 018 exists to solve, reached by a
-- path 018 could not see.
--
-- ---------------------------------------------------------------------
-- WHY NOT JUST STAMP THE STATBLOCK KEY ON THE ACTOR
-- ---------------------------------------------------------------------
--
-- Because it would have thrown away the name. Webbys' template carries
-- `npc_key = giant_spider`, so the existing branch would have numbered
-- it off the SPECIES - "Giant Spider 0001" - and then renamed the
-- character to that, deleting the name a DM chose on purpose. The whole
-- point of a template is that it is yours and it is named.
--
-- So: the actor records WHICH TEMPLATE it came from, and the base is
-- that template's name. "Webbys 0001", "Webbys 0002". The ordinal
-- counting is not reimplemented - it is the same three lines, reading
-- the same `name_base` across the same game, because a second copy of
-- that would be a second thing to keep true.
--
-- `on delete set null`, NOT CASCADE. Deleting a template must not reach
-- into a fight that was built from it - 123 says the creatures made
-- from a template are untouched by its deletion, and an actor is one of
-- those. `name_base` is already stored by then, so the naming survives
-- the source disappearing.

alter table public.encounter_actors
  add column if not exists template_id uuid
    references public.characters(id) on delete set null;

comment on column public.encounter_actors.template_id is
  'The template this actor was copied from, when it was copied from one (138). PROVENANCE, like npc_key beside it, and read by exactly one thing: name_actor numbers a copied creature off its template''s name, so three copies of Webbys are Webbys 0001 to 0003 rather than three actors called Webbys. Null for a character enrolled as themselves and for one instantiated from a reference statblock, which has npc_key instead.';

create index if not exists encounter_actors_template_idx
  on public.encounter_actors(template_id) where template_id is not null;

-- ---------------------------------------------------------------------
-- AND THE THIRD BRANCH
-- ---------------------------------------------------------------------
--
-- Byte-for-byte 018's function with one branch inserted and the ordinal
-- block shared between it and the statblock case. The order of the
-- branches is the order of specificity: an explicit label wins over
-- everything, then a template, then a statblock, then "they are
-- themselves".

create or replace function public.name_actor()
returns trigger
language plpgsql
set search_path to ''
as $function$
declare
  base text;
  n    integer;
  gid  uuid;
begin
  if new.label is not null and btrim(new.label) <> '' then
    return new;
  end if;

  -- 139. A COPY OF A TEMPLATE IS NUMBERED OFF THE TEMPLATE'S NAME, so
  -- the name the DM gave it survives and the copies can be told apart.
  if new.template_id is not null then
    select nullif(btrim(t.name), ''), e.game_id
      into base, gid
      from public.encounters e
      left join public.characters t on t.id = new.template_id
     where e.id = new.encounter_id;
    base := coalesce(base, 'Creature');

  elsif new.npc_key is not null then
    select nullif(btrim(concat_ws(' ', nullif(btrim(n2.species), ''),
                                       nullif(btrim(n2.class), ''))), ''),
           e.game_id
      into base, gid
      from public.encounters e
      left join public.npcs n2
        on n2.key = new.npc_key
       and (n2.game_id = e.game_id or n2.game_id is null)
     where e.id = new.encounter_id
     order by n2.game_id nulls last
     limit 1;

    if base is null then
      select nullif(btrim(n2.name), '') into base
        from public.npcs n2
       where n2.key = new.npc_key
       order by n2.game_id nulls last
       limit 1;
    end if;

    base := coalesce(base, 'Creature');

  else
    -- THEY ARE THEMSELVES. A player character, or a creature that was
    -- already standing in the world and is enrolled where it stands.
    select c.name into base
      from public.characters c where c.id = new.character_id;
    new.label := coalesce(nullif(btrim(base), ''), 'Adventurer');
    return new;
  end if;

  -- ONE ORDINAL RULE for both of the branches that need one: the next
  -- number after the highest this game has used for this base.
  select coalesce(max(ea.name_ordinal), 0) + 1 into n
    from public.encounter_actors ea
    join public.encounters e2 on e2.id = ea.encounter_id
   where e2.game_id = gid
     and ea.name_base = base;

  new.name_base    := base;
  new.name_ordinal := n;
  new.label        := base || ' ' || lpad(n::text, 4, '0');

  update public.characters
     set name = new.label
   where id = new.character_id;

  return new;
end;
$function$;

-- THE ONE ACTOR ALREADY STANDING IS LEFT ALONE, both its name and its
-- provenance. Webbys walked in before any of this existed.
--
-- Renaming it is clearly wrong: it is mid-fight in round 3, it has a
-- resolved attack with `character_name = 'Webbys'` snapshotted on two
-- roll rows, and 018's whole argument is that a name already rolled
-- under is a fact.
--
-- BACKFILLING THE ID IS WRONG TOO, which took a moment longer to see. A
-- character names the statblock it descends from, not the template, so
-- the only way back is to match `npc_key` against this game's templates
-- - and THERE ARE TWO GOBLIN TEMPLATES in Test Game 1. The match is not
-- unique, so the join would have let Postgres choose, and an arbitrary
-- answer written into a provenance column is worse than an empty one: a
-- null says "came in before 139", and a guess says "came from that
-- template" when nobody knows that. This is the fault this codebase is
-- named after, and a one-row convenience is not worth it.
