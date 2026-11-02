# Standing odyssey1e up on a new machine

**Read this from GitHub in a browser on the new laptop.** You do not
need the old machine to be running, reachable, or even alive to get
this project working.

**The one fact that makes this easy: nothing in this project is
secret.** The repo is public and the only credential in it is the
Supabase *publishable* key, which is designed to ship inside clients -
`src-tauri/src/supabase.rs` says so in a header. So there is no
`.env` to move, no key vault to unlock, no file you must rescue before
the old disk is wiped. **The clone is the whole application.**

Everything below is therefore one of three things: software to install,
an account to sign into, or a convenience you would otherwise spend an
afternoon rediscovering.

---

## 1. Install, in this order

| | what | version on the old machine | notes |
|---|---|---|---|
| 1 | **Git for Windows** | 2.55.0 | Brings Git Bash, which some of this project's tooling assumes |
| 2 | **Rust**, via [rustup.rs](https://rustup.rs) | 1.98.1 | **Say yes when it offers the Visual Studio C++ build tools.** See below |
| 3 | **Node**, LTS, from nodejs.org | 22.18.0 | No bundler. The frontend is plain files served out of `src/` |
| 4 | **Python 3** | 3.13.5 | Not needed to run the app. Needed for `tools/recover_migrations.py` and for the large-edit scripts this project's tooling pattern uses |

**The build-tools prompt is the one that bites.** If you skip it, the
first build fails much later with `link.exe not found`. That reads like
a Rust problem and is not - it is a missing Microsoft C++ toolchain. If
you miss the prompt, install "Desktop development with C++" from the
Visual Studio Build Tools installer.

**WebView2** ships with Windows 11, so there is nothing to do. On
anything older, install the Evergreen runtime.

### Worth having, not required

- **VS Code**, plus the two extensions this repo recommends -
  `tauri-apps.tauri-vscode` and `rust-lang.rust-analyzer`. They are in
  `.vscode/extensions.json`, so VS Code will offer them when you open
  the folder.
- **GitHub CLI** (`gh`). It was *not* installed on the old machine;
  worth adding, because PR work otherwise goes through the browser.
- **Claude Code**, if you want to carry on the way this project has
  been built. See §4.

---

## 2. Get it running

```powershell
git clone https://github.com/TirayisDM/odyssey1e.git
cd odyssey1e
.\run.ps1
```

`run.ps1` is the launcher and it already handles the two things that
otherwise waste time: it installs `node_modules` on a fresh clone, and
it kills a running instance that is holding the binary.

**The first build compiles the whole Tauri dependency tree - ten to
twenty minutes, mostly silent. It is not hung.** The script warns you.
Later runs take seconds.

Then confirm the checkout is sound:

```powershell
.\run.ps1 -Test
```

**1436 tests, about a second.** If that number is lower, you are on an
older commit; if tests *fail* on a clean clone, something is wrong with
the toolchain rather than the code.

```powershell
npm test
```

The renderer smoke test, from the repo root. It needs no database and
no sign-in, and **a map tab that draws nothing is the first thing to
check after any pull** - this answers it in a second.

`make-shortcut.ps1` puts a desktop shortcut in place if you want one.

---

## 3. Accounts to sign into

Nothing here is copied from the old machine. You sign in fresh.

| account | what it is for | without it |
|---|---|---|
| **GitHub** | pushing to `TirayisDM/odyssey1e` | you can still clone, build, run and test |
| **Supabase** | the `Odyssey RPG` org, project `odyssey1e`, ref `shraejtytdxmoxmxkwxq`, us-west-1 | you can write migration files but not apply them |
| **The app itself** | your own account and campaign | nothing - sign up in the app with your own email |
| **Claude Code** | how this project has been built | nothing, it is a tool choice |

Set your git identity on the new machine before the first commit:

```powershell
git config --global user.name "Dave Myers"
git config --global user.email "<the address the existing commits use>"
```

Check it against `git log` first. A mismatched email silently detaches
your new commits from your GitHub contribution history.

---

## 4. What does *not* travel in the clone

This section is the actual reason this file exists.

### The Supabase console settings

The auth hardening is **not in git and no migration captures it.** The
README's table is the only record:

| setting | value |
|---|---|
| Minimum password length | 10 |
| Password requirements | lower, upper, digits, symbols |
| Secure password change | on |
| Require current password when updating | on |

**Nothing to do if you keep the same Supabase project** - these live
server-side and the new laptop inherits them automatically. It matters
only if the project is ever rebuilt from migrations, which would start
from Supabase's defaults (length 6, no requirements, both guards off).
Those last two are load-bearing for a specific reason given in the
README: they are what stops a borrowed session becoming a permanent
takeover.

### Claude Code's memory for this project

Copy this folder across:

```
C:\Users\tiray\.claude\projects\C--Users-tiray-odyssey1e\memory\
```

to the same path under the new user name. It is two small files -
`MEMORY.md` (the index) and one memory holding the working-style rule
this project runs on. Nothing else under `.claude\` needs to move;
session transcripts are history, not configuration.

**If the old machine is already gone, this one is recoverable and you
have lost nothing.** The rule it holds is written down in `STATUS.md`,
under the heading *"The working style, which is now a rule"* - the
"ONE CYCLE, EXPLAINED, APPROVED, THEN BUILT" entry, along with the
incident that caused it. Rewrite the memory from that section rather
than treating the folder as lost. Like the auth table above - which
`README.md` records and `README.md` is in the clone - the thing that
does not travel is the *file*, not the knowledge in it.

### Your fast-login PIN - and do not copy this one

```
%APPDATA%\com.tirayis.odyssey1e\fast-login.json
```

**Set a new PIN in the app on the new machine instead.** Reasons in §5.

### The source material on `G:`, which is not in the repo at all

**"Pick up here" at the end of `STATUS.md` points at a Google Drive
path, not a tracked file:**

```
G:\My Drive\appsheet\raw backup
```

That is the Apps Script system this project is being ported from - 17
`.js` files, with `diceroller.js` and `characternarrative.js` the ones
still being read. `WeaponsAttacks.js` there is required reading before
the versatile-weapons cycle. **`G:\My Drive\appsheet\Application
Data.xlsx` is the data behind all of it**, 22 tabs, and its `_RAW` tab
is the honest source the seeds were generated from.

**So install the Google Drive client and sign in, or the next cycle
has nothing to read.** Nothing here is secret and nothing needs
rescuing from the old disk - it is in Drive, not on the laptop - but
the clone alone will not get you there, which is why it belongs in
this section. Check `G:` resolves before you start a porting cycle.

### What *does* travel, so stop worrying about it

- **All 60 tile images.** `src/assets/tiles/*.png` is tracked in git.
  `ART.md` is the brief if you add more.
- Every migration, the whole Rust and JS source, `STATUS.md`,
  `README.md`, `MAP.md`, the icons, `Cargo.lock` and
  `package-lock.json`.

---

## 5. Before the old laptop leaves your hands

**There is a plain-text credential on it.**
`%APPDATA%\com.tirayis.odyssey1e\fast-login.json` holds a **refresh
token**, and `pin.rs` says so out loud rather than pretending
otherwise: *anyone who can read that file can mint a session without
knowing the PIN.* The PIN is a convenience, not a lock.

So, in this order:

1. **On the old laptop:** clear the PIN in the app, or delete that
   file.
2. **From the new machine, once you are signed in:** change your app
   password. That invalidates refresh tokens, which closes the hole
   even if step 1 was missed or the file was already copied somewhere.
3. Sign out of GitHub, Supabase and Claude Code on the old machine.
4. **Then** wipe the disk. Not before - and a factory reset that keeps
   the user profile is not a wipe.

Also worth knowing: `C:\Users\tiray\.claude\` holds full transcripts of
the sessions that built this project. Nothing secret, but it is a
detailed record of your work, so wipe it with the rest rather than
leaving it on a machine you are passing on.

---

## 6. First-day sanity checks

Run these in order. Each one fails differently, which is the point.

| check | passing answer |
|---|---|
| `.\run.ps1 -Test` | 1436 tests, 0 failed |
| `npm test` | the map smoke test completes |
| `.\run.ps1`, then sign in | the eight tabs load |
| open a map, break a wall | a heap of rubble lands on the square |
| click **Lift what is underfoot** | it lifts, *given a shovel, spade or mattock in the pack* - otherwise it tells you what it wants |
| `select * from public.security_doors()` in the Supabase SQL editor | **an empty result is the pass.** Run it after any migration |

Then read the end of `STATUS.md`. **"Pick up here" is the end of the
log** and says what the next cycle is.

---

## 7. Gotchas that cost an hour each

- **The first build can fail before it compiles anything, and it is not
  `link.exe`.** If `run.ps1 -Test` dies in seconds - not minutes - with

  ```
  warning: spurious network error (3 tries remaining): [35] SSL connect error
  error: failed to get `rand` as a dependency of package `odyssey1e`
  CRYPT_E_NO_REVOCATION_CHECK (0x80092012)
  ```

  then **your network is fine and cargo never reached crates.io.**
  Windows schannel could not fetch certificate *revocation* data, and
  cargo's libcurl uses schannel. The tell is that `git clone` worked
  moments earlier - Git ships its own OpenSSL - while
  `curl.exe https://index.crates.io/config.json` fails identically.
  "spurious" is a lie; it reproduces every time.

  The fix is one file, `%USERPROFILE%\.cargo\config.toml`:

  ```toml
  [http]
  check-revoke = false
  ```

  **Know what that trades.** Chain, hostname and expiry are still
  verified and `Cargo.lock` checksums still hold; what stops is
  rejecting a *revoked* certificate. Delete the file to restore the
  default. Before reaching for it, rule out a TLS interceptor - see the
  `reqwest` comment in `src-tauri/Cargo.toml`, where Avast's Web Shield
  caused a neighbouring problem - because if something *is* standing in
  the middle, this setting hides it rather than fixing it.
- **Cargo commands run from inside `src-tauri`,** not the repo root.
  `src-tauri` is a standalone Cargo package.
- **`error: failed to remove file ...odyssey1e.exe - Access is denied.
  (os error 5)`** means a previous app window is still running and
  holding the binary. **It is not a code error.** `run.ps1` kills it
  for you; by hand it is
  `Get-Process odyssey1e | Stop-Process -Force`.
- **A Rust change needs the window closed and the command re-run.**
  Frontend edits hot-reload; Rust edits do not.
- **Rust arguments are `snake_case`, JS passes `camelCase`,** and Tauri
  converts between them. Getting that backwards is the standard
  first-day confusion. There are 166 commands, all registered in
  `lib.rs`.
- **Never put a service-role key or the database password in this
  repo.** A service-role key bypasses RLS completely. `supabase.rs`
  says that one appearing there is a bug, and the repo is public.

---

## 8. Bringing another developer on

Already covered in `README.md` under *Working on this with someone
else* - they need nothing secret, they sign up in the app with their
own email and get their own game. The three things that must be granted
rather than self-served are a GitHub collaborator invite, an invite to
the `Odyssey RPG` Supabase org, and a game's join code.

Note that **nothing in this application can read or reset a password.**
A collaborator locked out of their app account needs a reset from the
Supabase dashboard by whoever holds the project.
