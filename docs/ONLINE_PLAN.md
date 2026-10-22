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
