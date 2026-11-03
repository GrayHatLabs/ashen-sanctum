# itch.io page: Ashen Sanctum

The files for the page are in this folder:
- `cover_630x500.png`: the cover image.
- `banner_960x300.png`: the header image, under Edit theme.
- `screenshots/`: 1280×720 screenshots. Upload 5 to 10 of them.

## The "Create a new project" form

| Field | What to put |
|---|---|
| **Title** | `Ashen Sanctum` |
| **Project URL** | `https://grayhatlabs.itch.io/ashen-sanctum` |
| **Short description or tagline** | `A gothic isometric action RPG in the spirit of Diablo II: eight heroes, seven acts, and online co-op for up to eight players.` |
| **Classification** | Games |
| **Kind of project** | Downloadable |
| **Release status** | Released (or *In development* if you'd rather) |
| **Pricing** | No payments, or **$0 or donate** once payments are set up in your account |
| **Cover image** | `cover_630x500.png` |
| **Gameplay video** | (leave empty, or a YouTube link later) |
| **Screenshots** | `screenshots/01_coop.png` ... `10_town.png` |

### Uploads

Download these from the [v1.7.0 GitHub release](https://github.com/GrayHatLabs/ashen-sanctum/releases/tag/v1.7.0), upload each one,
and tick its platform:

| File | Tick |
|---|---|
| `AshenSanctum-windows.zip` | Windows |
| `AshenSanctum-macos.zip` | macOS |
| `AshenSanctum-linux-x86_64.tar.gz` | Linux |
| `AshenSanctum-android.apk` | Android |
| `AshenSanctum-handheld-aarch64.zip` | none (name it "Handheld (RG35XX, Odin, PortMaster)") |

### Details

- **Genre:** Role Playing (or Action)
- **Tags:** `action-rpg`, `hack-and-slash`, `isometric`, `pixel-art`, `dungeon-crawler`, `dark-fantasy`, `gothic`,
  `co-op`, `online-multiplayer`, `singleplayer`. Itch suggests these as you type.
- **AI generation disclosure:** Yes. Itch asks this for new projects. The art (sprites and paintings) was made
  with AI image tools (PixelLab, OpenAI), and the code was written with Claude. Itch lets you say which parts.
- **Input:** Keyboard, Mouse, Xbox controller, Playstation controller
- **Multiplayer:** Local and network. Under *Multiplayer*, tick *Server-based networked multiplayer*, *Ad-hoc networked multiplayer*, and *Supports up to 8 players*.
- **Languages:** English
- **Accessibility:** Configurable controls, a large text option
- **Links:** Source code: https://github.com/GrayHatLabs/ashen-sanctum
- **Community:** Comments
- **Visibility:** Draft first. Check the page, then switch it to Public.

## Description

Paste this into **Description**. The editor keeps headings and lists when you paste them.

---

**Ashen Sanctum** is a gothic, isometric action RPG in the spirit of Diablo II. The Ash King has risen
beneath Hollowmere. Choose a hero, gather the seals, and fight your way through seven acts: crypts and frozen
wastes, a haunted wood, a clockwork empire, a drowned sea floor, a broken sky, and the churning chaos at the
end of everything.

### Eight heroes
- **Sorceress**: fireballs, meteors and a phoenix of ash.
- **Vampire**: blood magic, bats and thralls that rise to fight for her.
- **Sky Pirate**: a heavy 1800s flintlock, a rapier, bombs and clockwork turrets.
- **Valkyrie**: frost spear, ravens, and a charge on her warhorse.
- **Berserker**: a giant axe, whirlwinds and a dire wolf.
- **Reaper**: rune scythe and souls, the Ledger of the dead.
- **Druid**: plague, spores and summons.
- **Inquisitor**: holy fire, chains and brands.

Every hero has a skill tree, set items, and ancient powers in the endgame.

### Seven acts and beyond
- Hand-designed bosses, each in a battle hall of their own, with tells you can learn and phases you can see on the bar.
- Side quests, secrets, super uniques, shrines, a bestiary, an arena and a rival who won't leave you alone.
- Nightmare and Hell difficulties, rekindled bosses, Ash Rifts and Embers for the endgame.
- Weather, water and lava, day-long dungeons, and loot: magic, rare, set, unique and ancient items, gems and sockets.
- A potion belt, a food bag, and a blood vial for the vampire, all upgraded through quests.

### Play together
- **Online co-op for up to 8 players.** Host a game for friends, or run your own login server (Docker).
- Everyone gets their own loot, and nearby kills share experience. Monsters grow tougher and more numerous with each hero.
- Joiners trade, talk to people and take side quests, chat with Enter, and move instantly with prediction.
- Everything is encrypted (TLS 1.3), and the host checks everything the players send.
- **Hardcore** heroes have one life. **Players 1-8** makes a solo game as hard and as rich as a full party.

### Plays anywhere
- Windows, macOS, Linux and Android.
- Handhelds: RG35XX, AYN Odin and others through PortMaster.
- Keyboard and mouse or a gamepad, with remappable buttons, a help menu, a how-to-play guide and a large text option.

### Free and open
The code is MIT licensed, and the art and music are CC BY-NC 4.0. The source is on
[GitHub](https://github.com/GrayHatLabs/ashen-sanctum).

---

## After publishing

- **New versions:** upload the new release files and remove the old ones. Butler (itch's command-line uploader)
  can automate this later; GitHub Actions could push each tagged release straight to itch.
- **Online play:** players need the same version as the host or server.
