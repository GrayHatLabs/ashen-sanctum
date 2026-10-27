# Running the Ashen Sanctum login server (Docker)

The login server is the game itself run with `--server`: no window, no sound. Players log in with an account,
keep their heroes on the server, and create or join games from a list. Nobody has to open ports on their own
router: only the server's machine needs one. (Security: [docs/ONLINE_PLAN.md](../docs/ONLINE_PLAN.md).)

## 1. Install Docker

- **Linux** (Fedora, Ubuntu...): Docker Engine and the Compose plugin, from docs.docker.com/engine/install.
- **Windows:** Docker Desktop.

## 2. Get the game and set it up

```bash
git clone https://github.com/GrayHatLabs/ashen-sanctum.git
cd ashen-sanctum
cp docker/.env.example docker/.env
```

Edit `docker/.env`:
- `ASHEN_INVITE`: a code people need to register. Make it long (12 characters or more) and give it only to
  your friends. The server won't start with the example's placeholder. Left empty, anyone who can reach the
  server can make an account.
- `ASHEN_SERVER_NAME`: what players see.

## 3. Start it

```bash
docker compose -f docker/docker-compose.yml up -d --build
```

The first build takes a few minutes. Then:

```bash
docker logs ashen-server
```

It shows a line like `ASHEN SANCTUM listening on port 4747; key 1717 BC03 8F11 273A 4F0A 863D`.
**Send that key to your players**: the first time they connect, the game shows the server's key and asks them
to check it.

## 4. Let players reach it

- **Same network:** players use this machine's local address (for example `192.168.1.50`).
- **Over the internet:** forward TCP port 4747 on your router to this machine, and allow it in the
  machine's firewall. On Fedora, for example:

  ```bash
  sudo firewall-cmd --add-port=4747/tcp --permanent && sudo firewall-cmd --reload
  ```

  Players then use your public address.

## In the game

**PLAY ONLINE**, then **USE A SERVER**:
1. Type the address and check the key.
2. Register (with the invite code) or log in.
3. Make a hero, then create or join a game.

The server and the players must run the same version of the game.

## Updating

```bash
git pull
docker compose -f docker/docker-compose.yml up -d --build
```

Accounts and heroes are kept: they live in the `ashen-sanctum_ashen-data` volume, not in the container.

## Backups

```bash
docker run --rm -v ashen-sanctum_ashen-data:/data:ro -v "$PWD":/backup debian:bookworm-slim tar czf /backup/ashen-data.tgz -C /data .
```

That file holds every account (password hashes only), every hero, and the server's private key. Keep it safe.

## What's locked down

- **Runs as a plain user,** not root.
- **A read-only filesystem**, except the data volume and a small /tmp.
- **No Linux capabilities and no privilege gain**, plus limits on memory and processes.
- **One port**, TLS 1.3 only.
- **Passwords:** hashed with Argon2id. Failed logins lock the account and the address for a while, and sign-ups are limited.
- **Logs** show who logged in and what games started, never passwords, codes or chat. They're rotated (3 files of 10 MB).
