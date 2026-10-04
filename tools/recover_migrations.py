#!/usr/bin/env python3
"""Write the migration files the database still has and the repo does not.

WHY THIS EXISTS
---------------
`supabase/migrations/` is the record of how this schema was built, and
for a while it was an incomplete one: migrations were applied to the live
project and the `.sql` file was written afterwards, which in practice
meant never. 0ed7491 recovered four of them (087, 092, 093, 094) by
hand. This is the same job as a script, so it is minutes rather than an
afternoon and so nobody has to do it by eye again.

WHAT IT RECOVERS
----------------
Supabase records every migration it applies in
`supabase_migrations.schema_migrations`, including the statements - with
the comment headers intact. So these come out as the files AS APPLIED
rather than as a reconstruction of what they probably said.

It only writes a file that is MISSING. An existing file is never
touched, so running this twice is safe and it can never overwrite
something you have edited.

VERIFIED, NOT TRUSTED. Every file written is read back and MD5-checked
against the database's own record before this reports success.

WHAT IT CANNOT RECOVER
----------------------
A migration applied through the SQL editor rather than the migration
API leaves no row here. Those are simply absent, and the script lists
them at the end as the gaps it could not fill. They have to be
reconstructed from the live schema by hand.

USAGE
-----
    pip install psycopg2-binary
    python tools/recover_migrations.py "<connection string>"

The connection string is in the Supabase dashboard under
Settings -> Database -> Connection string -> URI, with your database
password in it. It is a credential: do not paste it into a chat, a
commit, or this file. Pass it as an argument or set DATABASE_URL.

    export DATABASE_URL="postgresql://..."    # or $env:DATABASE_URL on Windows
    python tools/recover_migrations.py

Add --dry-run to see what it would write without writing anything.
"""

import hashlib
import os
import re
import sys
from pathlib import Path

try:
    import psycopg2
except ImportError:
    sys.exit(
        "psycopg2 is not installed.\n"
        "    pip install psycopg2-binary"
    )

HERE = Path(__file__).resolve().parent
MIGRATIONS = HERE.parent / "supabase" / "migrations"

# A migration this repo names: three digits, an underscore, a slug.
# The database also holds Supabase's own bookkeeping rows and migrations
# applied before this convention started, and neither belongs in the
# folder.
NUMBERED = re.compile(r"^(\d{3})_[a-z0-9_]+$")


def main() -> int:
    dsn = (sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].startswith("-")
           else os.environ.get("DATABASE_URL"))
    if not dsn:
        sys.exit(__doc__.split("USAGE")[1].strip())
    dry_run = "--dry-run" in sys.argv

    if not MIGRATIONS.is_dir():
        sys.exit(f"no migrations folder at {MIGRATIONS}")

    on_disk = {
        m.group(1): p.name
        for p in MIGRATIONS.glob("*.sql")
        if (m := NUMBERED.match(p.stem))
    }

    with psycopg2.connect(dsn) as conn, conn.cursor() as cur:
        cur.execute(
            """
            select name, array_to_string(statements, E'\\n'), version
              from supabase_migrations.schema_migrations
             where name is not null
             order by version
            """
        )
        rows = cur.fetchall()

    in_db = {}
    for name, sql, version in rows:
        m = NUMBERED.match(name)
        if m and sql:
            in_db[m.group(1)] = (name, sql, version)

    missing = sorted(set(in_db) - set(on_disk))
    if not missing:
        print(f"nothing to recover - {len(on_disk)} files, none missing from the database")
    written, failed = [], []

    for num in missing:
        name, sql, _ = in_db[num]
        path = MIGRATIONS / f"{name}.sql"
        want = hashlib.md5(sql.encode("utf-8")).hexdigest()

        if dry_run:
            print(f"  would write {path.name:<60} {len(sql):>6} chars")
            continue

        # newline="" so Python does not translate \n to \r\n on Windows -
        # a CRLF file would not match the MD5 and would be a different
        # file from the one that ran.
        path.write_text(sql, encoding="utf-8", newline="")

        got = hashlib.md5(path.read_text(encoding="utf-8", newline="").encode("utf-8")).hexdigest()
        if got == want:
            written.append(path.name)
            print(f"  {path.name:<60} {len(sql):>6} chars  md5 ok")
        else:
            failed.append(path.name)
            print(f"  {path.name:<60} MD5 MISMATCH - wrote {got}, wanted {want}")

    # THE GAPS NOTHING CAN FILL. A migration applied through the SQL
    # editor is not recorded here, so a number with no file and no row is
    # schema that exists in the database and is written down nowhere.
    highest = max([int(n) for n in set(on_disk) | set(in_db)] or [0])
    unrecoverable = [
        f"{n:03d}" for n in range(1, highest + 1)
        if f"{n:03d}" not in on_disk and f"{n:03d}" not in in_db
    ]

    print()
    if written:
        print(f"recovered {len(written)} migration(s), every one MD5-verified")
    if failed:
        print(f"FAILED on {len(failed)}: {', '.join(failed)}")
    if unrecoverable:
        print(
            f"still missing and NOT in the database's record ({len(unrecoverable)}):\n"
            f"  {', '.join(unrecoverable)}\n"
            "  These were applied by a route that records nothing. They have to be\n"
            "  reconstructed from the live schema by hand - `pg_dump --schema-only`\n"
            "  is the place to start."
        )
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
