//! Story: the quest line, the people of Hollowmere and their dialogue.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Elder,
    Merchant,
    Healer,
    Guard,
    Villager(u8),
    // ---- Kaldholm (Act 2) ----
    /// Captain Brenna: the Act 2 story.
    Captain,
    /// Old Sigurd: sells like Gerta.
    Trader,
    /// Mother Ylva: heals and resets skills like Brother Aldric.
    Seer,
    Fisher(u8),
}


pub struct Npc {
    pub name: &'static str,
    pub role: Role,
    pub art: &'static str,
    pub x: f32,
    pub y: f32,
    pub dir: usize,
    pub home: (f32, f32),
    /// Villagers stroll: (dx, dy, time left).
    pub wander: (f32, f32, f32),
    pub anim_t: f32,
    pub moving: bool,
}

impl Npc {
    pub fn new(name: &'static str, role: Role, art: &'static str, x: f32, y: f32, dir: usize) -> Self {
        Npc { name, role, art, x, y, dir, home: (x, y), wander: (0.0, 0.0, 2.0), anim_t: 0.0, moving: false }
    }
}

/// Where the story stands.
#[derive(Clone, Debug, Default)]
pub struct Quest {
    /// 0 = haven't met the elder, 1 = hunting the wardens, 2 = Sanctum unsealed, 3 = Ash King slain.
    pub stage: u8,
    /// Seals taken from the Bone, Plague and Hex Wardens.
    pub seals: [bool; 3],
    /// 0 normal, 1 nightmare, 2 hell (D2 style: the world gets harder, your hero carries on).
    pub difficulty: u8,
    /// Act 2: 0 = haven't met Captain Brenna, 1 = hunting the Frost Heralds, 2 = the glacier is open,
    /// 3 = the Rime Wyrm is slain.
    pub stage2: u8,
    /// Frost runes from the Frost Giant, the Yeti Matriarch and the Rime Witch.
    pub runes: [bool; 3],
}

pub const DIFFICULTIES: [&str; 3] = ["NORMAL", "NIGHTMARE", "HELL"];

impl Quest {
    pub fn seal_count(&self) -> usize {
        self.seals.iter().filter(|s| **s).count()
    }

    pub fn rune_count(&self) -> usize {
        self.runes.iter().filter(|s| **s).count()
    }

    /// Act 2 is open (the Ash King is dead).
    pub fn north_open(&self) -> bool {
        self.stage >= 3
    }

    /// One-line quest log for Act 2.
    pub fn log2(&self) -> String {
        match self.stage2 {
            0 => "FIND CAPTAIN BRENNA IN KALDHOLM".into(),
            1 if self.rune_count() < 3 => format!("SLAY THE THREE FROST HERALDS  ({}/3 RUNES)", self.rune_count()),
            1 => "BRING THE RUNES TO CAPTAIN BRENNA".into(),
            2 => "ENTER THE GLACIER'S HEART. SLAY THE RIME WYRM".into(),
            _ if self.difficulty < 2 => "THE WYRM IS DEAD. CAPTAIN BRENNA WANTS A WORD".into(),
            _ => "THE WYRM IS DEAD. THE NORTH IS FREE".into(),
        }
    }

    /// Does Captain Brenna have news (a marker over her head)?
    pub fn captain_has_news(&self) -> bool {
        self.stage2 == 0 || (self.stage2 == 1 && self.rune_count() == 3) || (self.stage2 == 3 && self.difficulty < 2)
    }

    /// One-line quest log for the HUD.
    pub fn log(&self) -> String {
        match self.stage {
            0 => "SPEAK WITH ELDER MAREN IN HOLLOWMERE".into(),
            1 if self.seal_count() < 3 => format!("SLAY THE THREE WARDENS  ({}/3 SEALS)", self.seal_count()),
            1 => "RETURN THE SEALS TO ELDER MAREN".into(),
            2 => "ENTER THE ASHEN SANCTUM. SLAY THE ASH KING".into(),
            _ => "THE ASH KING IS DEAD. TAKE THE PASS NORTH TO THE FROSTMARCH".into(),
        }
    }

    /// Does the elder have something new to say (shows a marker over her head)?
    pub fn elder_has_news(&self) -> bool {
        self.stage == 0 || (self.stage == 1 && self.seal_count() == 3)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Ware {
    HealthPotion,
    ManaPotion,
    Bread,
    Roast,
}

impl Ware {
    pub fn price(self) -> i32 {
        match self {
            Ware::HealthPotion | Ware::ManaPotion => 30,
            Ware::Bread => 12,
            Ware::Roast => 30,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Act {
    Next,
    Close,
    Buy(Ware),
    /// Reset skill points (Brother Aldric), for this much gold.
    Respec(i32),
    /// Waypoint travel.
    Travel(crate::world::LevelId),
    /// Gerta's gear (opens the inventory with her stock).
    Shop,
    /// After the Ash King: start the next difficulty.
    NextDifficulty,
}

pub struct Dialog {
    pub name: &'static str,
    pub pages: Vec<String>,
    pub page: usize,
    pub options: Vec<(String, Act)>,
    pub sel: usize,
    /// Quest stage to advance to once the conversation has been read.
    pub advance_to: Option<u8>,
    /// Heal the player when the dialog opens (Brother Aldric).
    pub heals: bool,
    /// Options for the last page of a multi-page conversation.
    pub last_options: Vec<(String, Act)>,
}

impl Dialog {
    fn new(name: &'static str, pages: &[&str]) -> Self {
        let pages: Vec<String> = pages.iter().map(|s| s.to_string()).collect();
        let mut d = Dialog { name, pages, page: 0, options: vec![], sel: 0, advance_to: None, heals: false, last_options: vec![] };
        d.refresh_options();
        d
    }

    /// Options for the current page: NEXT until the last page, then the NPC's own options.
    pub fn refresh_options(&mut self) {
        if self.page + 1 < self.pages.len() {
            self.options = vec![("CONTINUE".into(), Act::Next)];
        } else if !self.last_options.is_empty() {
            self.options = self.last_options.clone();
        } else if self.options.iter().all(|o| o.1 == Act::Next) {
            self.options = vec![("FAREWELL".into(), Act::Close)];
        }
        self.sel = self.sel.min(self.options.len().saturating_sub(1));
    }
}

/// What an NPC says, given the story so far.
pub fn talk(role: Role, q: &Quest) -> Dialog {
    match role {
        Role::Elder => match q.stage {
            0 => {
                let mut d = Dialog::new(
                    "ELDER MAREN",
                    &[
                        "AH, A WANDERER. FORGIVE AN OLD WOMAN'S STARE. FEW COME TO HOLLOWMERE SINCE THE ASH BEGAN TO FALL.",
                        "BENEATH THE ASHEN SANCTUM SLEEPS THE ASH KING, ONCE ARCHMAGE OF THIS LAND. HE SOUGHT TO BURN AWAY DEATH ITSELF, AND BURNED INSTEAD.",
                        "THREE WARDENS GAVE THEIR LIVES TO SEAL HIM. NOW THEIR TOMBS ARE DEFILED, AND THE WARDENS WALK AGAIN, TWISTED TO HIS WILL.",
                        "SLAY THE BONE WARDEN IN THE CRYPT TO THE SOUTHWEST, THE PLAGUE WARDEN IN THE WARRENS TO THE SOUTHEAST, AND THE HEX WARDEN IN THE CATACOMBS TO THE NORTHEAST.",
                        "BRING ME THEIR SEALS. WITH ALL THREE, THE SANCTUM GATE IN THE NORTHWEST CAN BE OPENED, AND THE ASH KING ENDED. THE CRYPT IS THE LEAST DANGEROUS. BEGIN THERE.",
                    ],
                );
                d.advance_to = Some(1);
                d
            }
            1 if q.seal_count() == 3 => {
                let mut d = Dialog::new(
                    "ELDER MAREN",
                    &[
                        "ALL THREE SEALS... I CAN FEEL THEIR WARMTH FROM HERE. THE WARDENS ARE AT PEACE AT LAST.",
                        "I HAVE SPOKEN THE OLD WORDS. THE ASH BARRIER ON THE SANCTUM GATE IS BROKEN.",
                        "GO NOW, AND END HIM. THE SANCTUM LIES TO THE NORTHWEST. HOLLOWMERE WILL REMEMBER YOUR NAME.",
                    ],
                );
                d.advance_to = Some(2);
                d
            }
            1 => {
                let line = format!(
                    "THE WARDENS STILL WALK. YOU HOLD {} OF 3 SEALS. GERTA SELLS POTIONS AND FOOD, AND BROTHER ALDRIC WILL MEND YOUR WOUNDS.",
                    q.seal_count()
                );
                Dialog::new("ELDER MAREN", &[&line])
            }
            2 => Dialog::new("ELDER MAREN", &["THE SANCTUM AWAITS, TO THE NORTHWEST. MAY THE FLAME GUIDE YOU, CHILD."]),
            _ => Dialog::new(
                "ELDER MAREN",
                &[
                    "THE ASH HAS STOPPED FALLING. FOR THE FIRST TIME IN YEARS, CHILDREN PLAY IN THE SQUARE. THANK YOU, SORCERESS.",
                    "BUT FEEL THAT WIND... THE ASH KING'S FIRE HELD BACK THE COLD OF THE NORTH FOR AN AGE. NOW THE SNOWS ARE COMING SOUTH.",
                    "THE MOUNTAIN PASS NORTH OF HOLLOWMERE IS OPEN AGAIN. BEYOND IT LIES THE FROSTMARCH, AND THE TOWN OF KALDHOLM. THEY WILL NEED YOU.",
                ],
            ),
        },
        Role::Merchant => {
            let mut d = Dialog::new("GERTA THE MERCHANT", &["GERTA'S GOODS! COIN FOR COMFORT. WHAT'LL IT BE?"]);
            d.options = vec![
                (format!("HEALING POTION  {} GOLD", Ware::HealthPotion.price()), Act::Buy(Ware::HealthPotion)),
                (format!("MANA POTION  {} GOLD", Ware::ManaPotion.price()), Act::Buy(Ware::ManaPotion)),
                (format!("LOAF OF BREAD  {} GOLD", Ware::Bread.price()), Act::Buy(Ware::Bread)),
                (format!("ROAST  {} GOLD", Ware::Roast.price()), Act::Buy(Ware::Roast)),
                ("SHOW ME YOUR GEAR (AND BUY MINE)".into(), Act::Shop),
                ("LEAVE".into(), Act::Close),
            ];
            d
        }
        Role::Healer => {
            let mut d = Dialog::new(
                "BROTHER ALDRIC",
                &["BE STILL, CHILD... THE LIGHT MENDS WHAT THE DARK HAS TORN. RETURN WHENEVER YOU ARE HURT."],
            );
            d.heals = true;
            d
        }
        Role::Captain => match q.stage2 {
            0 => {
                let mut d = Dialog::new(
                    "CAPTAIN BRENNA",
                    &[
                        "A SOUTHERNER? THROUGH THE PASS? THEN THE ASH KING IS TRULY DEAD. WE FELT HIS FIRE DIE. WE FELT THE COLD COME AFTER IT.",
                        "SOMETHING HAS WOKEN BENEATH THE GLACIER. VORTHRAX, THE RIME WYRM. A WHITE DRAGON OLDER THAN ANY KINGDOM. HER BREATH FREEZES RIVERS SOLID.",
                        "THREE FROST HERALDS SERVE HER: A FROST GIANT IN THE MINES TO THE WEST, THE YETI MATRIARCH IN THE CAVES TO THE NORTHEAST, AND THE RIME WITCH IN HER TEMPLE TO THE NORTHWEST.",
                        "EACH HOLDS A FROST RUNE. TOGETHER THEY UNSEAL THE GLACIER'S HEART, FAR TO THE NORTH. BRING ME ALL THREE, SORCERESS. YOUR FIRE IS THE ONE THING THEY FEAR.",
                    ],
                );
                d.advance_to = Some(11);
                d
            }
            1 if q.rune_count() == 3 => {
                let mut d = Dialog::new(
                    "CAPTAIN BRENNA",
                    &[
                        "ALL THREE RUNES. THEY ARE SO COLD THEY BURN TO HOLD.",
                        "MOTHER YLVA HAS SUNG THE OLD SONG OVER THEM. THE SEAL ON THE GLACIER'S HEART IS BROKEN. GO NORTH, AND KILL THE WYRM BEFORE THE LAKE FREEZES TO ITS BED.",
                    ],
                );
                d.advance_to = Some(12);
                d
            }
            1 => {
                let line = format!(
                    "THE HERALDS STILL STAND. YOU HOLD {} OF 3 RUNES. OLD SIGURD TRADES FOR GEAR, AND MOTHER YLVA WILL MEND YOU.",
                    q.rune_count()
                );
                Dialog::new("CAPTAIN BRENNA", &[&line])
            }
            2 => Dialog::new("CAPTAIN BRENNA", &["THE GLACIER'S HEART LIES FAR TO THE NORTH. DRESS WARM, AND BURN HER, SORCERESS."]),
            _ if q.difficulty < 2 => {
                let next = DIFFICULTIES[q.difficulty as usize + 1];
                let mut d = Dialog::new(
                    "CAPTAIN BRENNA",
                    &[
                        "THE WYRM IS DEAD. THE ICE ON THE LAKE IS SINGING AS IT THAWS. THE SKALDS WILL SING OF YOU FOR A HUNDRED WINTERS.",
                        "AND YET MOTHER YLVA DREAMS OF ASH AND ICE TOGETHER. THE ASH KING STIRS IN THE SOUTH, THE WYRM'S BONES IN THE NORTH. THEY WILL RISE AGAIN, AND STRONGER.",
                        "IF YOU WOULD FACE THEM ONCE MORE, THE WORLD WILL BE HARDER, BUT ITS TREASURES RICHER. YOU KEEP ALL YOU HAVE LEARNED AND CARRY.",
                    ],
                );
                d.last_options = vec![(format!("BEGIN {next}"), Act::NextDifficulty), ("NOT YET".into(), Act::Close)];
                d
            }
            _ => Dialog::new("CAPTAIN BRENNA", &["EVEN HELL COULD NOT HOLD YOU. ASH AND ICE ARE BOTH BROKEN. GO IN PEACE, SORCERESS."]),
        },
        Role::Trader => {
            let mut d = Dialog::new("OLD SIGURD", &["FURS, FOOD, AND STEEL THAT DOESN'T SHATTER IN THE COLD. SOUTHERN GOLD SPENDS THE SAME UP HERE."]);
            d.options = vec![
                (format!("HEALING POTION  {} GOLD", Ware::HealthPotion.price()), Act::Buy(Ware::HealthPotion)),
                (format!("MANA POTION  {} GOLD", Ware::ManaPotion.price()), Act::Buy(Ware::ManaPotion)),
                (format!("LOAF OF BREAD  {} GOLD", Ware::Bread.price()), Act::Buy(Ware::Bread)),
                (format!("ROAST  {} GOLD", Ware::Roast.price()), Act::Buy(Ware::Roast)),
                ("SHOW ME YOUR GEAR (AND BUY MINE)".into(), Act::Shop),
                ("LEAVE".into(), Act::Close),
            ];
            d
        }
        Role::Seer => {
            let mut d = Dialog::new(
                "MOTHER YLVA",
                &["SIT BY THE FIRE, CHILD. THE COLD GETS INTO THE BONES, AND I KNOW THE SONGS THAT DRAW IT OUT."],
            );
            d.heals = true;
            d
        }
        Role::Fisher(k) => {
            let lines = [
                "THE FISH WON'T BITE. THEY HIDE AT THE BOTTOM OF THE LAKE, LIKE THEY'RE AFRAID OF SOMETHING UNDER THE ICE.",
                "RAIDERS FROM THE HILLS BURNED THE NORTH FARMS LAST WINTER. THEY SAY THE GIANT IN THE MINES PAYS THEM IN SILVER.",
                "STAY OFF THE OPEN SNOW AT NIGHT. THE WINTER WOLVES HUNT IN PACKS OF A DOZEN, AND THE YETIS EAT WHAT THEY LEAVE.",
            ];
            Dialog::new(if k == 2 { "FISHERWIFE" } else { "FISHERMAN" }, &[lines[k as usize % 3]])
        }
        Role::Guard => {
            let line = if q.stage == 0 {
                "HALT- OH, A TRAVELLER. ELDER MAREN WILL WANT TO SEE YOU. SHE'S BY THE FIRE."
            } else if q.seal_count() == 0 {
                "WOLVES AND GOBLINS ROAM THE WILDS. STAY ON THE ROADS. THEY SAY GOBLINS FLEE WHEN THEIR KIN FALL. COWARDS, THE LOT."
            } else {
                "THEY SAY YOU FELLED A WARDEN. THE WHOLE TOWN IS TALKING. KEEP IT UP."
            };
            Dialog::new("CAPTAIN ROLF", &[line])
        }
        Role::Villager(i) => {
            let lines = [
                "THE ASH GETS INTO EVERYTHING. MY BREAD TASTES OF SMOKE.",
                "MY BROTHER WENT INTO THE CRYPT LOOKING FOR TREASURE. HE NEVER CAME BACK.",
                "IF YOU'RE HUNGRY, GERTA SELLS BREAD. WILD APPLES GROW OUT IN THE WOODS, TOO.",
                "AT NIGHT YOU CAN SEE RED LIGHT FROM THE SANCTUM, OFF TO THE NORTHWEST.",
            ];
            Dialog::new(if i % 2 == 0 { "VILLAGER" } else { "FARMER" }, &[lines[(i as usize) % lines.len()]])
        }
    }
}

/// The ending, shown after the Ash King falls.
pub const EPILOGUE: [&str; 4] = [
    "THE ASH KING FALLS.",
    "HIS CROWN OF HORNS CRACKS, AND THE EMBERS IN HIS EYES GO DARK.",
    "ACROSS THE LAND THE ASH STOPS FALLING. IN HOLLOWMERE, THE BELLS RING FOR THE FIRST TIME IN YEARS.",
    "A COLD WIND BLOWS DOWN FROM THE NORTH. THE MOUNTAIN PASS IS OPEN...",
];

pub const EPILOGUE2: [&str; 4] = [
    "VORTHRAX THE RIME WYRM FALLS.",
    "THE GLACIER GROANS AND CRACKS, AND MELTWATER RUNS SOUTH FOR THE FIRST TIME IN AN AGE.",
    "IN KALDHOLM THE FISHERMEN DANCE ON THE THAWING LAKE. ASH AND ICE ARE BROKEN.",
    "THANK YOU FOR PLAYING ASHEN SANCTUM.",
];

/// Breaks text into lines of at most `width` characters on word boundaries.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = vec![];
    let mut cur = String::new();
    for word in text.split_whitespace() {
        if !cur.is_empty() && cur.len() + 1 + word.len() > width {
            lines.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(word);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_respects_width() {
        let lines = wrap("THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG", 10);
        assert!(lines.iter().all(|l| l.len() <= 10));
        assert_eq!(lines.join(" "), "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG");
    }

    #[test]
    fn elder_advances_the_quest() {
        let mut q = Quest::default();
        assert_eq!(talk(Role::Elder, &q).advance_to, Some(1));
        q.stage = 1;
        assert_eq!(talk(Role::Elder, &q).advance_to, None);
        q.seals = [true; 3];
        assert_eq!(talk(Role::Elder, &q).advance_to, Some(2));
    }
}
