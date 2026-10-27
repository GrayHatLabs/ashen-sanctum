# Online play plan: host-and-join, then a login server

Status: **plan** (2026-10-21). The Players 1–8 scaling (v1.5.9) is built and is the first piece.

Decided:
- **Both**: host-and-join first (one player hosts, friends join by address, like D2's TCP/IP games), then a
  **dedicated login server** on top (accounts, heroes stored on the server, a game list, like Battle.net).
- **TCP**, client and server.
- Monsters scale with the real number of players in the game, using the same rules as the Players setting
  (`world::build_for(id, seed, difficulty, players)`; `binds::players_scale`).

## How it works

**The host runs the world.** One machine (the hosting player, or later the server) runs the real game:
monsters, damage, loot, every hero. The others send it their **inputs** (move here, cast this skill there,
pick that up). Many times a second it sends back **snapshots** of what's near each player (heroes, monsters,
missiles, pickups, effects). Clients draw the snapshots and smooth the motion between them.

The other way, lockstep (every machine runs the same game in sync), was ruled out. It needs bit-for-bit
identical maths on Windows, macOS, Linux and the ARM handhelds, and `sin`/`cos` differ between them. Snapshots
don't care, and they stop cheating by modified clients.

## Phase 1: many heroes in one game (no network yet)

The biggest job. Today the game has exactly one hero: `Game.p`, used in about 3,200 places.
- `Game.p` becomes `Game.heroes: Vec<Player>`, plus "the local hero" (an index) for the camera, HUD, bag and menus.
- Monsters choose a target: the nearest hero, or whoever hurt them.
- Skills, missiles, auras and damage carry who cast them, so kills, XP and Valor/souls/blood go to the right hero.
- XP is shared within range (like D2's party XP). **Loot is per player:** each drop belongs to one hero; only
  that hero's player sees it or can pick it up.
- Death: a fallen hero respawns in town (softcore). Hardcore heroes play only in hardcore games, and die for good.
- Towns, quests and waypoints: quest progress follows the host's game, and each hero keeps their own.
- **Testing without a network:** a second hero driven by the existing bot, in the same process. The bot
  survey and the map check keep working.

### Phase 1 progress (2026-10-22): the core is in (src/party.rs)

Done:
- `HeroState` holds everything one hero owns: the hero, bag, skills, timers, and every missile, summon and
  class effect in flight. `swap_hero(k)` trades an ally into the active slot and back, so the game's code works
  unchanged for any hero. The leader is hero 0; allies are 1–7.
- Each tick runs the leader, then the shared world, then each ally (their own input, skills and missiles).
- Monsters target the nearest living hero outside town (re-checked twice a second). Blows, missiles and ground
  hazards land on whichever hero they reach.
- Allies are drawn with their name and a life bar. Everyone's missiles and effects are drawn.
- The leader leads: allies come along through every portal. A fallen ally gets up beside the leader after 6 s.
- Experience: every hero within 25 tiles of a kill gets it. Loot: every hero there gets their **own** roll
  (their own magic find and class set); only they see it or can take it. Gold, potions, food and quest items
  are shared. The Players setting's extra drop chance only counts pretend players, not real ones.
- Tests: `an_ally_shares_the_kill_and_gets_loot_of_its_own`. The bot runs with allies
  (`ASHEN_ALLIES=sorceress,reaper,vampire`): five heroes through Acts 1, 3 and 5 with no deaths or crashes.
  Solo play is unchanged (the survey numbers match).

Still to do in phase 1 (or early phase 2):
- World triggers that watch "the player" (shrines, set pieces, errands, tides, the darkness) answer only the
  leader so far.
- Opening the skill tree or the bag pauses the world, which can't happen online.
- Allies' summons follow the leader; monsters don't push off allies; the Druid's wolf follows only the leader.
- Per-hero menus, dialogs and shops (today allies can't talk to people).
- Hardcore allies (hardcore games).

## Phase 2: host and join over TCP

- **Menu:** Play → *Host online game* / *Join online game* (enter an address, or pick from a LAN list found by
  broadcast), plus a lobby showing who's in, each hero's class and level, and a Start button.
- **Wire format:** length-prefixed messages over one TCP connection per client, serialised with `serde`
  (already a dependency; compact binary via `bincode` or `postcard`). `TCP_NODELAY` on.
- **Messages:**
  - Client → host: hello (name, version, hero), inputs about 30×/s, chat.
  - Host → client:
    - Welcome: the level seed and difficulty, so the client builds the same map itself rather than receiving it.
    - Snapshots about 20×/s: only what's near that player, and only what changed.
    - Events: sounds, floaters, notices, music.
- **On the client:** draws from snapshots, moves its own hero ahead of the snapshots so walking feels instant
  (corrected when the host disagrees), and keeps its bag, skills and stats in step with the host's copy.
- **Saves:** in a hosted game each player's hero comes from their own save and is saved back when they leave.
  That's trusting, which is fine among friends.
- **Internet play:** the host must open a port (default **4747**) on their router. Phase 3's server removes that.
- **Handhelds:** can join over Wi-Fi. Hosting from them is optional (they're slower).

## Phase 3: the login server

- **The same program run headless** (`ashensanctum --server`), so there's no second codebase. It can run on the
  TrueNAS in Docker, or on a small cloud box.
- **Accounts:** a username and password. Passwords are hashed (`argon2`), and the connection is encrypted
  (TLS via `rustls`) so passwords never cross the internet in the clear.
- **Server-side heroes:** stored on the server (like D2's realm characters), so saves can't be edited.
  Offline heroes stay separate.
- **Game list:** create a game (name, optional password, difficulty, max players), browse and join.
  The server runs each game itself, so nobody needs to open ports.
- **Admin:** a simple config file, logs, and a ban list.

## Phase 4: polish

- Chat (Enter to type; a phrase wheel on the pad).
- Party frames (others' life on the left, like D2), names over heroes, and pings on the minimap.
- Trading between players.
- Rejoin after a dropped connection.
- Bandwidth and lag tests (a fake-lag switch for testing).

## Security (a requirement for every phase)

The user asked for best security practice throughout. These are rules for the code, not options.

**Never trust the client**
- The host or server runs everything. Clients only send *intentions* (move toward, use skill N at a point,
  pick up item id), never results (damage, gold, XP, item stats, positions).
- Every input is checked:
  - Values in range and finite (no NaN or infinity).
  - Skills known, off cooldown and affordable; targets within reach and in sight.
  - Items exist, are the player's own and are within pickup range.
  - Movement no faster than the hero can go.
  - Anything invalid is dropped and counted; too many and the client is disconnected.
- In host-and-join, a joining hero is checked before it's accepted: level, stats and items possible for its
  level, no impossible affixes. Phase 3 removes this risk entirely with server-side heroes.

**Robust network code**
- A hard cap on message size (e.g. 64 KB) read from the length prefix *before* allocating. A per-connection
  buffer limit, bounded queues, and back-pressure.
- Timeouts on every stage: handshake, login, idle, slow readers and writers. Limits on connections per IP and
  in total, and on messages per second per connection.
- **No panics on network data:** no `unwrap`/`expect`/indexing on anything decoded. Every decode returns an
  error and drops the message (or the client).
- `#![forbid(unsafe_code)]` in the network modules.
- A protocol version in the handshake; mismatches are refused cleanly.
- Decoding through a bounded binary format (`postcard` or `bincode` with size limits), never unbounded JSON
  from the network.
- **Fuzz** the message decoder (`cargo-fuzz`) and add tests for malformed, truncated, oversized and flood input.

**Accounts and login (phase 3)**
- Passwords hashed with **Argon2id** (a per-user random salt, current recommended settings). Never stored,
  logged or sent anywhere in plain text.
- All login-server traffic over **TLS 1.3** (`rustls`), with a real certificate (Let's Encrypt) or a pinned
  certificate shipped with the game. No plaintext fallback.
- Session tokens: 256 random bits from the OS's secure random source. They expire, are revoked on logout,
  and only a hash of each is stored on the server.
- Rate limits and growing delays on failed logins (per account and per IP). The same error message whether
  the username or the password was wrong. Constant-time comparisons.
- Username rules (length, allowed characters). Files and records are keyed by **account id**, never by a
  name, so there's no path traversal. Database access through parameterised queries only.

**Host-and-join (phase 2)**
- An optional game password. The connection is encrypted with TLS too: the host makes a key on first run, and
  joiners see its fingerprint and trust it on first use, like SSH.
- LAN discovery replies give only the game name, player count and version. Nothing connects automatically.
- Hardcore and online heroes are kept apart from edited offline saves.

**Server deployment**
- Runs as a non-root user in Docker: only its data folder writable, one port open, no shell tools needed.
- Logs never contain passwords, tokens or chat contents by default. Other players' IP addresses are never shown.
- Backups of the account database. A ban list (account and IP).

**Chat and names**
- Length limits, control characters stripped, rate-limited. Text is shown only through the game's own font
  renderer (no markup to inject).

**Supply chain**
- `cargo audit` and `cargo deny` in CI (known vulnerabilities and licences), with locked dependency versions.
- As few dependencies as possible, only well-maintained crates (rustls, argon2, postcard).
- A security review of each phase's changes before it's released.

### Security review of phase 2 (2026-10-22)

An independent review of `src/net/` and `src/online.rs` found 11 issues; all are fixed:
1. **A host could crash joiners** (an unchecked table index while drawing, and `panic = "abort"` disabled the guard).
   - Fixed by:
     - Index checks.
     - Release builds now unwind, and the joiner's whole frame and draw are guarded.
     - Every incoming level reference (levels, portals, waypoints, town portal) and pickup index is validated.
     - Float sorts use `total_cmp`.
     - Drawing primitives clip to the screen, so a huge radius can't stall a joiner.
   - **A fuzz test** (`mangled_snapshots_never_crash_a_joiner`): 3,000 corrupted snapshots per seed are applied and drawn; 12 seeds were clean.
2. **A slow handshake could hold slots forever.** The handshake now has an overall deadline (`tls::handshake`).
3. **Edited saves could cheat.** Item stats are capped by the game's own roll tables (`items::stat_ok`), and skill points, gold, xp and level are checked.
4. **A host could hang a joining game.** Join and probe have absolute deadlines; the welcome is read frame by frame.
5. **A host could flood a joiner.**
   - The inbound queue is bounded (512).
   - Only the newest snapshot is used.
   - At most 4 levels in 2 seconds.
   - Levels inflate to at most 8 MB.
6. **A host could damage a joiner's save.** A lower-level copy is never merged, and the last five saves are kept (`.bak1`-`.bak5`).
7. **Chat floods.** Each joiner gets 2 lines a second (5 at once); the rest isn't relayed.
8. **Password lockout gaps.** Tries are counted per address (per /64 for IPv6), and expired entries are pruned.
9. **Trust on first use.**
   - `--trust` needs the host's key.
   - known_hosts is keyed by the address as typed.
   - (On Windows the key sits in %APPDATA%, private to the account by default.)
10. **Snapshots leaked other heroes' details.** Other players get a public view of each hero: no bag, stash, waypoints or records.
11. **A failed connection thread leaked its slot.** The slot is now given back.

### After v1.6.0

- **v1.6.1: chat.** Enter opens a line.
- **Joiners and people.** Joiners talk, trade, use the jeweler and take side quests:
  - The host runs only the choices a joiner may make.
  - Shop and jeweler actions count only near where they were opened.
  - The host leads all travel.
- **Movement prediction.** Keys and the stick move a joiner's hero at once. Each snapshot carries the last input the host applied (`ack`), and the joiner replays its unconfirmed steps on top, working off differences (snapping if over 2.5 tiles). The host holds a joiner's last input between packets, and one-shot buttons are used once.

### Phase 3: built (2026-10-27)

- **`ashensanctum --server`** (`src/server/`): accounts (`accounts.rs`), the lobby on each connection (`mod.rs`), and game rooms on their own threads (`room.rs`).
  - Every hero is a remote player. The first player leads (and may travel); the next takes over if the leader leaves.
  - Heroes are saved to their account (hero only) every minute and on leaving. Hardcore deaths are saved as fallen.
- **Accounts:**
  - Passwords hashed with Argon2id (OWASP settings), with the same answer and the same work for unknown names.
  - Locks after failed logins (per account, growing; and per address), sign-up limits, an optional invite code.
  - At most two hashes at once. Files keyed by account number and written atomically; deleted heroes are kept in `deleted/`.
- **Limits:**
  - Connections per address and in all, a 10 s handshake deadline, and a 60 s lobby idle timeout (clients ping every 15 s).
  - 3 failed logins per connection, 12 hero or game changes a minute, a maximum number of games, 8 players a game, and one hero in one game at a time.
- **Docker** (`docker/`): runs as a non-root user with a read-only root, no capabilities, no-new-privileges, memory and process limits, rotated logs, and a named data volume.
- **In the game:** Play Online, then Use a Server (`src/lobby_ui.rs`).
- **Shared art:** decoded once per process and shared between rooms (`Arc`). The party size is per thread, so each room scales its own monsters.

## Order of work and size

1. Phase 1: several sessions. It touches most of game.rs and the class files; the tests and bot keep it safe.
2. Phase 2: a few sessions for the protocol, lobby and smoothing, then testing on two PCs and a handheld.
3. Phase 3: a few sessions for the server mode, accounts, TLS and Docker.
4. Phase 4: as wanted.

New dependencies: `bincode` or `postcard` (phase 2); `argon2`, `rustls` (phase 3).

## Decisions (2026-10-22)

1. **Max players: 8**, like D2, matching the Players 1–8 setting.
2. **Loot: each player gets their own drops.** A drop belongs to one hero and only that player sees it or can pick
   it up, so nobody steals loot. (Phase 1: pickups get an owner; the host sends each client only its own.)
3. **No PvP.** Co-op only.
4. **Hardcore: separate hardcore games.** Hardcore heroes play online only with other hardcore heroes; death is
   still permanent.
5. **Login server hosting: decide later.** Build it to run anywhere in Docker on Linux (a cloud VPS or the TrueNAS).
6. **Heroes:** hosted games among friends use your **normal saved heroes**; the login server uses
   **separate server-stored heroes** that can't be edited.
