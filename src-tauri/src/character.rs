//! Character sheet: loading it, and turning a named request into dice.
//!
//! This is the port of getRollerContext() + resolveRequest() from
//! diceroller.js, split in two on purpose:
//!
//!   load_sheet()      talks to the network. Untestable without one.
//!   resolve_request() is pure. Given a Sheet it is arithmetic and
//!                     string matching, so it gets real unit tests.
//!
//! That split is the whole reason the rules are worth having in Rust.
//! In the old system resolution was tangled up with reading spreadsheet
//! ranges, so the only way to check "does Insight give +6" was to roll
//! and squint at the card.
//!
//! WHAT IS NOT HERE YET: techniques, spells, weapon attacks, death
//! saves, rests. Those were the back half of resolveRequest and each
//! needs its own table. A request that matches none of the cases below
//! falls through to "treat it as a raw formula", exactly as the original
//! did, so `2d6+3` still works and nothing is lost by the gap.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::dice::d20_formula;
use crate::equipment::{self, Owned};
use crate::narrative;
use crate::supabase;

/* ============================ THE SHEET ============================ */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ability {
    /// EFFECTIVE, species bonus already in it. Everything that derives
    /// a modifier reads this and therefore gets the bonus for free -
    /// skills, saves, attacks, carrying, AC. One place to apply it.
    pub score: i64,
    /// What is actually STORED in character_abilities - the number
    /// somebody rolled or bought. THE EDITOR MUST WRITE THIS ONE: the
    /// bonus is never folded into the row, or changing species later
    /// would double-count and an 18 would be indistinguishable from a
    /// 16 with a species behind it. See species.rs.
    pub base: i64,
    /// The species' contribution, for a sheet that wants to show its
    /// working. Zero when there is no species or none for this ability.
    ///
    /// EVERY SOURCE'S TOTAL, as of 091 - it was the species' alone,
    /// and the comment here said that when a second kind arrived this
    /// would have to become the sum. It has: an Ability Score
    /// Improvement is the second kind.
    ///
    /// KEPT ALONGSIDE `sources` because several callers read one number
    /// and none of them wants a list; the list is for the screen that
    /// shows the working.
    pub bonus: i64,
    /// WHERE THE BONUS CAME FROM, named. 080.
    ///
    /// A sheet showing `15 / 17` has to be able to say WHY, and "+2"
    /// on its own is an assertion rather than an explanation. One entry
    /// per contributing source, in the order they apply.
    ///
    /// ONE SOURCE EXISTS TODAY - the species. Class features, ability
    /// score improvements and items are all sources in 5e and none of
    /// them is modelled yet; this is a list rather than a second named
    /// field so that adding one is a push rather than a schema change
    /// and a screen change.
    pub sources: Vec<AbilitySource>,
    pub save_prof: bool,
}

/// One named contribution to an ability score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilitySource {
    /// What to call it on screen - the species' name, and later a
    /// class feature's or an item's.
    pub name: String,
    /// Signed. Negative is as real as positive: 5e has no species that
    /// subtracts, and exhaustion, curses and a dozen items do.
    pub value: i64,
}

impl Ability {
    /// A score with no species behind it: base and effective the same.
    /// Most callers and every fixture want this; `apply_species` is the
    /// only thing that ever moves them apart.
    pub fn plain(score: i64, save_prof: bool) -> Self {
        Self { score, base: score, bonus: 0, sources: Vec::new(), save_prof }
    }
}

/// A skill as the sheet shows it: what the character is worth before
/// anything was carried, what carried it, and the total.
///
/// THREE FACTS BECAUSE THE SCREEN SHOWS THREE. A +5 Perception on a
/// Wisdom 12 character is a number to take on faith; "+3, Amulet +2,
/// +5" is an account of itself, and the sheet has spelled out its
/// arithmetic everywhere else since the attack preview.
#[derive(Debug, Clone, Serialize)]
pub struct SkillLine {
    /// Ability and proficiency, with nothing worn counted.
    pub natural: i64,
    /// Each enchantment that moved it, named - `grants::breakdown`.
    pub parts: Vec<(String, i64)>,
    /// What to roll with.
    pub total: i64,
}

/// One named thing that moves a saving throw.
///
/// 119. A LABEL IS THE WHOLE POINT. The ability rows were about to grow
/// a "Save Mods" column, and a column of bare numbers is the thing this
/// sheet has refused to show since the attack preview spelled out STR
/// and proficiency separately.
#[derive(Debug, Clone, Serialize)]
pub struct SaveMod {
    /// What to call it: an item's name, a people's name, a spell's.
    pub name: String,
    /// A flat number, where it is one.
    pub value: Option<i64>,
    /// 114. A DIE, where the thing granting it rolls - Bless is +1d4
    /// on every save and is not a +2. Both may be present.
    pub dice: Option<String>,
    /// 118. TRUE FOR A BONUS THAT ONLY APPLIES AGAINST A SPELL, which
    /// the Ny'ook have. Shown either way and counted only when the
    /// circumstance is ticked - a conditional bonus folded silently
    /// into the total would be wrong on most saves, and one left out
    /// entirely is the invisibility 118 just finished fixing.
    pub only_vs_spell: bool,
}

/// One saving throw as the sheet shows it.
///
/// THE SAME THREE FACTS `SkillLine` CARRIES, for the same reason: what
/// the character is worth unaided, what moved it and by how much, and
/// the number to roll with.
///
/// `total` COUNTS THE UNCONDITIONAL MODIFIERS ONLY. A Ny'ook's +2 is
/// real and is not part of a bare Constitution save, so it rides in
/// `mods` with its flag set and the screen adds it when the
/// circumstance is ticked. Dice are never in the total - "+1d4" is not
/// a number and `effect_grants` appends it to the formula at roll time
/// so the roll shows what it rolled.
#[derive(Debug, Clone, Serialize)]
pub struct SaveLine {
    /// The ability modifier and, where they are proficient, the
    /// proficiency bonus. What a bare save rolls.
    pub natural: i64,
    /// Whether the proficiency bonus is in `natural`, so the screen can
    /// say so without recomputing the test.
    pub proficient: bool,
    /// Everything that moves it, named.
    pub mods: Vec<SaveMod>,
    /// `natural` plus every unconditional flat modifier.
    pub total: i64,
}

impl SaveLine {
    /// What the conditional modifiers come to - the Ny'ook's +2, and
    /// anything else that applies only against a spell.
    ///
    /// SEPARATE FROM `total` so one line can serve both rolls. A save
    /// against a dragon's breath and a save against Hold Person are the
    /// same character and different numbers.
    pub fn conditional(&self) -> i64 {
        self.mods
            .iter()
            .filter(|m| m.only_vs_spell)
            .filter_map(|m| m.value)
            .sum()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDef {
    pub key: String,
    pub name: String,
    /// Ability code this skill keys off: str/dex/con/int/wis/cha.
    pub ability: String,
}

/// What a character's Karma comes to, and the working behind it.
///
/// 076. CARRIED WITH THE PARTS SHOWING, because a number nobody can
/// check is a number nobody trusts. A bard looking at "Karma 8" should
/// see that it is Insight +3 and Performance +5 without opening a
/// rulebook, and a DM who thinks it is wrong should be able to say
/// WHICH half is wrong.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Karma {
    /// Which class expressed it. A Fighter/Bard has one class with a
    /// formula and one without, and this says which answered.
    pub class_key: String,
    /// The sum, clamped to the chart's 0-25 - see karma::rating.
    pub rating: i64,
    /// Before the clamp. Shown so a bard pinned at the top of the
    /// table can see how far past it they actually are.
    pub raw: i64,
    /// (skill key, that skill's modifier), in the order the class
    /// names them.
    pub parts: Vec<(String, i64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sheet {
    /// Where they are standing - 035, carried out so a screen can name
    /// the floor a drop is about to land on before it asks.
    pub location_id: Option<String>,
    /// NONE FOR A MONSTER. A goblin rolls from a sheet like everyone
    /// else — scores, a proficiency bonus, a loadout — but it is not a
    /// character and has no row in `characters`. `rolls.character_id` is
    /// nullable for exactly this, and the label travels in
    /// `character_name` instead, the same way 421d08c fixed death saves.
    pub character_id: Option<String>,
    pub game_id: String,
    /// Whose sheet this is: the character's name, or the actor's label
    /// for an NPC instance — "Goblin 0001".
    pub name: String,
    /// THE TOTAL, across every class. 073 made that a sum rather than a
    /// column: a Fighter 5 / Rogue 3 is level 8, and it is the 8 that
    /// buys their proficiency bonus. `characters.level` carries it,
    /// kept in step by `sync_character_level`, so nothing downstream
    /// has to add anything up to find out how good somebody is.
    pub level: i64,
    /// What they are, in the order they took it - 073. Empty for every
    /// monster and for a character made before 055, which is a real
    /// answer and not a gap.
    ///
    /// THE SHEET CARRIES KEYS AND LEVELS, NOT NAMES. The screen already
    /// holds the class catalogue to fill its dropdowns, and sending a
    /// display name here would be a second copy of it that goes stale
    /// the moment a DM renames a class.
    pub classes: Vec<crate::multiclass::Taken>,
    /// STATED rather than derived, for a monster. A character computes
    /// this from level because levelling is the rule that moves it; a
    /// statblock simply has one. None means derive — see
    /// `proficiency_bonus`.
    pub prof_bonus: Option<i64>,
    /// Ability code -> score and save proficiency. Always six entries;
    /// the database seeds them on character creation.
    pub abilities: HashMap<String, Ability>,
    /// The skill catalogue in effect for this character's game, with
    /// game-scoped overrides already resolved over the global rows.
    pub skills: Vec<SkillDef>,
    /// Skill key -> proficiency multiplier. Absent means untrained.
    pub profs: HashMap<String, f64>,
    /// Skill key -> the finished modifier: ability, proficiency, and
    /// any enchantment on what they are wearing.
    ///
    /// SENT BECAUSE THE SCREEN WAS WORKING IT OUT ITSELF. main.js
    /// carried `abilMod + floor(prof * pb)` inline, which was right
    /// until an item could change it - and then Falon's amulet
    /// granted +3 Perception, the roll took it, the Karma line took
    /// it, and the skills list went on printing the number it had
    /// always computed. Four places derived a skill modifier and
    /// three of them asked the engine.
    ///
    /// NOT DESERIALISED. Nothing parses a Sheet back, and the map is
    /// derived from the rest of it.
    #[serde(skip_deserializing)]
    pub skill_mods: HashMap<String, SkillLine>,
    /// Which narrative_lines pack voices this character's roll cards.
    pub narrative_pack: String,
    /// Roll key -> the lines available for it, pack precedence and game
    /// overrides already resolved. Read once with the sheet so choosing
    /// a line at roll time costs nothing — see narrative.rs.
    ///
    /// NOT SENT TO THE FRONTEND. A full pack is 180 lines, and the
    /// webview has no use for any of them — roll_named picks the line on
    /// this side and the chosen one travels on the roll row. Serializing
    /// it would put ~20KB of prose through the IPC boundary on every
    /// get_sheet. `default` keeps Deserialize working, since skipping a
    /// field on the way out says nothing about the way in.
    #[serde(skip_serializing, default)]
    pub narratives: HashMap<String, Vec<String>>,
    /// Foundry `traits.weaponProf.value`: `sim`, `mar`, or a bare
    /// baseItem granting one weapon. Empty by default, because a wrong
    /// proficiency is worse than an absent one — it moves to-hit and
    /// says nothing.
    pub weapon_profs: Vec<String>,
    /// Foundry `traits.armorProf.value`: lgt med hvy shl, the same
    /// spelling 008 normalizes `items.armor_category` to.
    pub armor_profs: Vec<String>,
    /// 075. Tools this character is trained with, by `items.key`. An
    /// instrument is the only kind so far, and this is empty for
    /// everybody who existed before there were any.
    pub tool_profs: Vec<String>,
    /// 076. What this character's Karma comes to, and from what.
    ///
    /// NONE FOR ALMOST EVERYBODY, which is an answer and not a gap.
    /// Karma is expressed per class and the bard is the only class
    /// that has one, so this is absent for every fighter, every
    /// monster and every classless character.
    pub karma: Option<Karma>,
    /// 056. The whole people, not just the key: the sheet shows the
    /// bonuses and the traits, and the viewer shows the prose. None
    /// when this character has no species, which is every one made
    /// before 056 and every monster.
    pub species: Option<crate::species::Species>,
    /// 058. What this particular person looks like, against the band
    /// their people occupies.
    pub body: Body,
    /// The size this character CARRIES as, Powerful Build included.
    /// Separate from `vitals.size`, which is the size they ARE - only
    /// carrying moves, not reach or cover or what a container admits.
    pub carry_size: Option<String>,
    /// What is equipped right now, with proficiency and attack modes
    /// already derived. Unlike `narratives` this one IS sent out — a
    /// loadout is a handful of rows and a sheet screen wants it.
    pub loadout: Vec<Owned>,
    /// The house techniques the EQUIPPED weapons offer, level gate and
    /// all. Read once with the sheet for the same reason the narrative
    /// pack is: choosing one at roll time should cost nothing.
    ///
    /// SENT OUT NOW. This was skipped both ways with the note that a
    /// technique picker could have them when one existed; the equipment
    /// panel is that picker. Only the EQUIPPED weapons' techniques are
    /// here - 31 rows in the whole table - so this is a handful of
    /// small objects, not the 180-line problem `narratives` was.
    ///
    /// OUTWARD ONLY. `skip_deserializing` rather than `default`, because
    /// Sheet derives Deserialize and `default` still demands the field
    /// be deserializable - it supplies a value when one is absent, it
    /// does not excuse the trait. Technique has no reason to implement
    /// it: nothing parses a Sheet back.
    #[serde(skip_deserializing)]
    pub techniques: Vec<crate::attack::Technique>,
    /// HP, death saves, exhaustion, size. 010.
    pub vitals: Vitals,
    /// 119. Ability code -> the saving throw, with its working.
    ///
    /// SENT BECAUSE THE SHEET IS ABOUT TO ROLL ONE. The ability rows
    /// carried a proficiency checkbox and no number at all, so a
    /// Constitution save was invisible until somebody typed it into the
    /// roll box - and the screen must not derive it itself, which is
    /// the fault `skill_mods` exists to have already fixed once.
    ///
    /// NOT DESERIALISED, like `skill_mods`: derived from the rest.
    #[serde(skip_deserializing)]
    pub save_lines: HashMap<String, SaveLine>,
    /// 122. WHAT THIS IS, in 5e's fourteen - and the one of species,
    /// role and type that spells are written against. Hold Person wants
    /// a humanoid; Cure Wounds refuses a construct or the undead.
    ///
    /// RESOLVED, NOT STORED. The creature's own word, then its people's,
    /// then its statblock's - `creature::of` owns that order. None is a
    /// real answer and means nobody has said, which is not humanoid.
    #[serde(skip_deserializing)]
    pub creature_type: Option<String>,
    /// 116. WHAT HURTS THEM LESS, MORE, OR NOT AT ALL, and WHY.
    ///
    /// DERIVED FROM FOUR PLACES AND STORED IN NONE. A species states
    /// its own; an item, a class feature and a running spell each say
    /// so with a grant. `species.damage_resistances` has existed since
    /// 056 and the sheet printed it followed by "(DM applies)", which
    /// was an honest label for a thing that did nothing.
    ///
    /// EVERY ENTRY NAMES ITS SOURCES. A sheet saying "resistant to
    /// fire" invites the question "why", and a barbarian who stops
    /// raging must lose the Rage half without taking the Unt'garoth
    /// half with it.
    ///
    /// NOT DESERIALISED, like `skill_mods`: nothing parses a Sheet
    /// back and this is derived from the rest of it.
    #[serde(skip_deserializing)]
    pub resistances: Vec<crate::resist::Standing>,
    /// DERIVED, NEVER STORED. What it takes to hit this character,
    /// computed from the loadout every time the sheet is read.
    ///
    /// The export's `flat: 14` is not this number and must not be
    /// mistaken for it — Rodnar computes to 15. Storing an AC would put
    /// it one equipment change away from being wrong, which is the
    /// mistake the Attacks tab made with to-hit.
    pub armor_class: i64,
    /// WHETHER THIS IS A CREATURE OFF A STATBLOCK, which is the one
    /// fact a technique gate needs and the sheet did not carry.
    ///
    /// True exactly when `characters.npc_key` names an `npcs` row - so
    /// an instantiated goblin and 123's template both say yes, and
    /// every player character says no. See `gate_level`, which is the
    /// only thing that reads it.
    pub from_statblock: bool,
    /// 150. WHO IS CASTING, AND OFF WHAT - None for almost everybody.
    ///
    /// RESOLVED ONCE, HERE. Six commands used to find a cleric class on
    /// the sheet and refuse anybody else, which is why a Lich could not
    /// cast a spell: a monster has no classes at all. A class says so or
    /// a statblock does, and `casting::caster` owns which wins.
    #[serde(skip_deserializing)]
    pub caster: Option<crate::casting::Caster>,
    /// 133. THE LEVEL A TECHNIQUE GATE READS, which is not always this
    /// character's level - `attack::gate_level` owns the rule and this
    /// is its answer, carried so that the three things which gate a
    /// move cannot disagree about it.
    ///
    /// SENT TO THE SCREEN for the reason `skill_mods` is: the picker
    /// compared `min_level` against `level` itself, so a creature
    /// would have had its own moves greyed out while the engine rolled
    /// them perfectly well.
    #[serde(skip_deserializing)]
    pub gate_level: i64,
    /// 154. WHAT THIS CHARACTER COULD CAST RIGHT NOW, so that a spell
    /// name typed into the roll box resolves the way a weapon name does.
    ///
    /// PREPARED AND CANTRIPS ONLY. A spell sitting in the book is held
    /// and not up today, so it is not a thing to roll - 107's three
    /// states are the whole of that rule and `load_sheet` applies it.
    ///
    /// EMPTY FOR EVERYONE WHO DOES NOT CAST, and loaded only when
    /// `caster` is Some - see the note on `load_sheet`'s query count.
    /// A fighter pays nothing for this field existing.
    #[serde(skip_deserializing)]
    pub spells: Vec<crate::spellcast::Known>,
}

impl Sheet {
    /// floor((level-1)/4)+2. Computed here rather than stored, so the
    /// rule lives in exactly one place.
    ///
    /// Unless the sheet states one. A monster's proficiency bonus comes
    /// off its statblock and has nothing to do with class levels, so a
    /// stated value wins — that is not an exception to "rules live in
    /// Rust", it is a different fact. The level formula is still the
    /// only place the LEVELLING rule is written down.
    pub fn proficiency_bonus(&self) -> i64 {
        match self.prof_bonus {
            Some(b) => b,
            None => ((self.level - 1) / 4) + 2,
        }
    }

    /// floor((score-10)/2). div_euclid, not plain division: Rust
    /// truncates toward zero and JS's Math.floor does not, so a score of
    /// 7 must give -2 and not -1.
    pub fn ability_mod(&self, code: &str) -> i64 {
        ability_mod_of(&self.abilities, code)
    }

    pub fn save_prof(&self, code: &str) -> bool {
        self.abilities.get(code).map(|a| a.save_prof).unwrap_or(false)
    }

    /// One saving throw, with everything that moves it named.
    ///
    /// 119. THE ONE PLACE A SAVE IS WORKED OUT. `resolve_request` had
    /// this arithmetic inline and the sheet had none at all, so a
    /// Constitution save was invisible until somebody typed it into the
    /// roll box. Giving the screen its own copy would be the fault
    /// `skill_mods` already exists to have fixed once - four places
    /// derived a skill modifier and three of them asked the engine.
    ///
    /// `live` IS WHAT IS RUNNING ON THEM, and the caller decides
    /// whether to pass any. The SHEET passes them, so Bless shows in
    /// the Save Mods column; `resolve_request` passes NOTHING, because
    /// `effect_grants` appends those dice to the formula at roll time
    /// and counting them here as well would roll Bless twice.
    ///
    /// A CONDITIONAL BONUS IS LISTED AND NOT TOTALLED. The Ny'ook +2
    /// applies only against a spell, so it rides in `mods` with its
    /// flag set - folding it into `total` would be wrong on most saves
    /// and leaving it out entirely is the invisibility 118 fixed.
    pub fn save_line(&self, code: &str, live: &[crate::grants::Grant]) -> SaveLine {
        let proficient = self.save_prof(code);
        let natural = self.ability_mod(code)
            + if proficient { self.proficiency_bonus() } else { 0 };
        let worn = worn_grants(&self.loadout);
        let both = format!("save.{}", code);

        // TWO TARGETS, IN THE ORDER `resolve_request` APPLIES THEM. A
        // Cloak of Protection grants `save` and a steadying ring grants
        // `save.dex`; both land, and a floor set by either settles
        // before the adds on top of it - see `grants::apply`.
        let mut mods = Vec::new();
        for (name, value) in crate::grants::breakdown(natural, &worn, "save") {
            mods.push(SaveMod { name, value: Some(value), dice: None, only_vs_spell: false });
        }
        let after_save = crate::grants::apply(natural, &worn, "save");
        for (name, value) in crate::grants::breakdown(after_save, &worn, &both) {
            mods.push(SaveMod { name, value: Some(value), dice: None, only_vs_spell: false });
        }
        let total = crate::grants::apply(after_save, &worn, &both);

        // WHAT IS RUNNING ON THEM, where the caller wanted it counted.
        // A die rather than a number, so the chip reads "Bless +1d4"
        // and nothing tries to add it to anything.
        for g in live.iter().filter(|g| g.target == "save" || g.target == both) {
            mods.push(SaveMod {
                name: g.source.clone(),
                value: (g.value != 0).then_some(g.value),
                dice: g.dice.clone(),
                only_vs_spell: false,
            });
        }

        // 118. AND THE ONE THAT ONLY APPLIES AGAINST A SPELL.
        if let Some(sp) = &self.species {
            if let Some(n) = sp.spell_save_bonus.filter(|n| *n != 0) {
                mods.push(SaveMod {
                    name: sp.name.clone(),
                    value: Some(n),
                    dice: None,
                    only_vs_spell: true,
                });
            }
        }

        SaveLine { natural, proficient, mods, total }
    }

    /// The proficiency contribution for a skill. floor(multiplier * PB),
    /// matching Math.floor(s.prof * ctx.pb) — so half-proficiency at PB 3
    /// is +1, not +1.5.
    pub fn skill_prof_bonus(&self, key: &str) -> i64 {
        let mult = self.profs.get(key).copied().unwrap_or(0.0);
        (mult * self.proficiency_bonus() as f64).floor() as i64
    }

    /// One skill's modifier: the ability behind it plus proficiency at
    /// whatever degree this character holds it.
    ///
    /// 076. NAMED RATHER THAN REPEATED. This pairing was already
    /// written inline in `resolve`, and Karma needs the same answer -
    /// so it is one method now and `resolve` calls it. A second copy
    /// is a second answer, and the second one is always the stale one.
    /// 100's skill grants are applied HERE, not at the call site, and
    /// the first live one is why. Boots that grant `skill.prf` were
    /// wired into `resolve_request` so a Performance ROLL picked them
    /// up - and `derive_karma` reads this function instead, so a
    /// bard's Karma went on being computed from the unenchanted
    /// number. Two answers for one skill, which is the fault this
    /// codebase keeps producing and the reason a rule belongs at the
    /// bottom of the stack rather than at whichever top somebody
    /// happened to be looking at.
    pub fn skill_modifier(&self, key: &str) -> i64 {
        match self.find_skill(key) {
            Some(s) => crate::grants::apply(
                self.skill_modifier_natural(key),
                &worn_grants(&self.loadout),
                &format!("skill.{}", s.key),
            ),
            None => 0,
        }
    }

    /// The same without anything magical in it.
    ///
    /// SPLIT OUT FOR THE KARMA LINE, which shows its working and must
    /// be able to say "Performance +1 + Pipes +1" rather than folding
    /// the two into a +2 nobody can account for. Everything else
    /// wants the total and calls `skill_modifier`.
    pub fn skill_modifier_natural(&self, key: &str) -> i64 {
        match self.find_skill(key) {
            Some(s) => self.ability_mod(&s.ability) + self.skill_prof_bonus(&s.key),
            None => 0,
        }
    }

    /// This character's Karma, if any class they hold expresses one.
    ///
    /// PER CLASS AND NEVER SUMMED. 073 lets somebody be two things; a
    /// Fighter 5 / Bard 3 has Bard Karma when acting as a bard, and
    /// their fighter levels do not touch it. Where more than one class
    /// eventually has a formula this takes the one that LEADS - which
    /// is the order `classes` already arrives in - rather than adding
    /// them, because two Karmas added is not a bigger Karma. It is a
    /// different question nobody has asked yet.
    ///
    /// EXPERTISE ARRIVES ON ITS OWN. `skill_prof_bonus` applies the
    /// stored degree, so a doubled Performance comes back doubled
    /// without this knowing what expertise is.
    fn derive_karma(&self, catalogue: &[crate::class::Class]) -> Option<Karma> {
        // THE FILTER IS INSIDE THE SEARCH, and that is the whole of
        // this line. Finding the first class and THEN asking whether it
        // has a formula gives up at the first one that does not - so a
        // Fighter 4 / Bard 1 reads as having no Karma at all, because
        // the fighter leads and fighters have none. It has to keep
        // looking past the classes that cannot answer.
        let c = self.classes.iter().find_map(|held| {
            catalogue
                .iter()
                .find(|c| c.key == held.key && !c.karma_skills.is_empty())
        })?;

        // EVERY CONTRIBUTION IS ITS OWN TERM, and named. A pair of
        // enchanted pipes is not part of somebody's Performance - it
        // is a thing they are carrying - so the line reads "Insight
        // +1 + Performance +1 + Pipes +1" rather than quietly
        // reporting a +2 the bard cannot account for.
        //
        // THE LABELS ARE RESOLVED HERE because this is where the
        // skill catalogue is. The screen used to turn a key into a
        // name and now prints what it is given, which is also what
        // lets an item sit in the same list as a skill.
        let worn = worn_grants(&self.loadout);
        let mut parts: Vec<(String, i64)> = Vec::new();
        for k in &c.karma_skills {
            let natural = self.skill_modifier_natural(k);
            let name = self
                .find_skill(k)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| k.clone());
            parts.push((name, natural));
            parts.extend(crate::grants::breakdown(
                natural,
                &worn,
                &format!("skill.{}", k),
            ));
        }
        let each: Vec<i64> = parts.iter().map(|(_, m)| *m).collect();
        Some(Karma {
            class_key: c.key.clone(),
            rating: crate::karma::rating(&each),
            raw: each.iter().sum(),
            parts,
        })
    }

    fn find_skill(&self, needle: &str) -> Option<&SkillDef> {
        let n = needle.trim().to_lowercase();
        self.skills
            .iter()
            .find(|s| s.key.to_lowercase() == n || s.name.to_lowercase() == n)
    }
}

/* ============================ RESOLUTION ============================ */

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Resolved {
    /// What the card says: "Insight (WIS)", "WIS Save".
    pub label: String,
    /// What the dice engine is handed.
    pub formula: String,
    /// The modifier that went into it, so a UI can explain the number
    /// instead of just showing it.
    pub modifier: i64,
    /// THE ROLL KIND, in engine vocabulary: a skill key ("ins"),
    /// "wis_save", "wis_check", or "custom" for a raw formula. Not the
    /// request the player typed — "Insight", "insight" and "ins" all
    /// resolve to the same key.
    ///
    /// This is the vocabulary narrative_lines.key and skill_prompts.key
    /// are written in, so it is what a narrative lookup and, later, an
    /// AI prompt seed are found by. The full vocabulary is emitted here
    /// even where no lines are seeded for it yet: a pack that grows a
    /// "wis_save" key then works without a Rust change.
    pub key: String,
    /// Present only for an attack, and carrying its own evidence: which
    /// ability was used, whether the proficiency bonus was applied, the
    /// damage formula, and the crit range in force.
    ///
    /// A to-hit of +1 where another weapon gives +5 reads as a bug
    /// unless the card can say the character is untrained with it, which
    /// is the same requirement the equipment panel already meets.
    pub attack: Option<crate::attack::Attack>,
}

/// Strip a "this save is against a spell" tail off a request, giving
/// back the save on its own.
///
/// FOUR SPELLINGS, because a person typing at a table types what they
/// say out loud and none of these is wrong. Singular and plural of
/// both prepositions covers everything anybody has written so far; a
/// fifth is a line here rather than a shrug on screen.
fn strip_vs_spell(t: &str) -> Option<&str> {
    for tail in [" vs spell", " vs spells", " against spell", " against spells"] {
        if let Some(rest) = t.strip_suffix(tail) {
            return Some(rest);
        }
    }
    None
}

/// Long and short ability names, both accepted. Ported from ABIL_NAMES.
fn ability_code(s: &str) -> Option<&'static str> {
    match s.trim().to_lowercase().as_str() {
        "str" | "strength" => Some("str"),
        "dex" | "dexterity" => Some("dex"),
        "con" | "constitution" => Some("con"),
        "int" | "intelligence" => Some("int"),
        "wis" | "wisdom" => Some("wis"),
        "cha" | "charisma" => Some("cha"),
        _ => None,
    }
}

/// Turn a request into a label and a formula.
///
/// Order matters and mirrors the original: saves, then skills, then
/// ability checks, then fall through to a raw formula. A skill named
/// "Athletics" must not be shadowed by anything, and an unknown request
/// must stay usable as a formula.
pub fn resolve_request(sheet: &Sheet, request: &str, mode: &str) -> Resolved {
    let t = request.trim().to_lowercase();

    // AN ATTACK FIRST. A weapon or technique name is the most specific
    // thing a request can be, and none of them collide with a skill or
    // an ability. Declining is cheap and silent, so the skills below are
    // reached exactly as before for everything that is not a swing.
    if let Some(a) = crate::attack::resolve(
        request,
        &sheet.loadout,
        &sheet.techniques,
        // NOT `sheet.level` - a creature is not gated by a ladder it
        // cannot climb. See attack::gate_level.
        sheet.gate_level,
        sheet.proficiency_bonus(),
        // 134. ANY of the six, because a weapon may name its own - the
        // sheet is the thing that knows all six and the attack path
        // never did.
        &|code| sheet.ability_mod(code),
    ) {
        return Resolved {
            label: crate::attack::label(&a),
            formula: d20_formula(a.to_hit, mode),
            modifier: a.to_hit,
            // One key for every weapon and technique. skill_prompts
            // already has its `attack` row seeded, and narrative_lines
            // can grow one without a Rust change.
            key: "attack".to_string(),
            attack: Some(a),
        };
    }

    // THEN A SPELL, for the same reason an attack comes first: a spell
    // name is a specific thing and collides with no skill or ability.
    //
    // 154. THE SECOND HALF OF CASTING, which was missing. `cast_spell`
    // deliberately does not roll - it spends the slot, writes the action
    // and puts the spell's name in the roll box, because an attack spell
    // is a d20 like any other and belongs in the one place that makes
    // them. Nothing here knew what that name meant, so the request fell
    // through to the raw-formula fallback and the dice engine was handed
    // the literal text "spiritual weapon". Luci cast it, the slot went,
    // the log said "+6 to hit, 1d8", and no attack was ever rolled.
    //
    // ONLY AN ATTACK SPELL RESOLVES HERE and `attack_for` owns that
    // rule. A Save spell is the TARGET's roll - the engine does not roll
    // for somebody else's character, which is why casting one reports
    // the DC and stops - and a Utility spell rolls no d20 at all.
    //
    // THE KEY IS `attack`, NOT `spell_attack`, and that is the same
    // decision the save branch below records: the key is the vocabulary
    // `narrative_lines` and `skill_prompts` are written in, and a key
    // nobody has seeded silently loses a character their prose. A spell
    // attack IS an attack roll against an AC, so it takes the prose that
    // already exists for one.
    if let Some(caster) = sheet.caster.as_ref() {
        if let Some(known) = crate::spellcast::find(&sheet.spells, request) {
            if let Some(a) = crate::spellcast::attack_for(
                known,
                sheet.proficiency_bonus(),
                sheet.ability_mod(&caster.ability),
                &caster.ability,
            ) {
                return Resolved {
                    // THE LABEL THE CAST ALREADY SAID, so the log line
                    // and the roll agree about what was promised.
                    label: crate::spellcast::label(&crate::spellcast::cast(
                        &known.key,
                        &known.name,
                        known.level,
                        &known.cast_type,
                        known.casting_time.as_deref(),
                        known.save_ability.as_deref(),
                        known.dice.as_deref(),
                        sheet.proficiency_bonus(),
                        sheet.ability_mod(&caster.ability),
                    )),
                    formula: d20_formula(a.to_hit, mode),
                    modifier: a.to_hit,
                    key: "attack".to_string(),
                    attack: Some(a),
                };
            }
        }
    }

    // "<ability> save", and the same save with what it is AGAINST:
    // "wis save vs spell", "dex save against spells".
    //
    // THE CIRCUMSTANCE IS PART OF THE REQUEST. 098 gave a people a
    // bonus that applies only against a spell, and nothing in a bare
    // `wis save` says whether a spell is casting it. Every other
    // conditional trait in this schema stays a DM call for exactly
    // that reason - Stonecunning doubles on SOME stonework and no
    // request says which. This one is different only because the
    // player knows at the moment they roll, and the request has always
    // been what they know.
    //
    // THE KEY DOES NOT FORK. It stays `wis_save`, because the key is
    // the vocabulary narrative_lines and skill_prompts are written in
    // and a `wis_save_spell` nobody seeded would silently lose a
    // character their prose.
    let (stem, vs_spell) = match strip_vs_spell(t.as_str()) {
        Some(rest) => (rest, true),
        None => (t.as_str(), false),
    };
    if let Some(stem) = stem.strip_suffix(" save") {
        if let Some(code) = ability_code(stem) {
            // 119. ASKED, NOT REPEATED. The ability mod, the
            // proficiency bonus and 100's two grant targets were
            // worked out here and nowhere else, so the sheet could
            // show a proficiency checkbox and no number. One method
            // owns it now and this is a caller - the same move
            // `skill_modifier` made when four places derived a skill.
            //
            // NOTHING LIVE IS PASSED. `effect_grants` appends a running
            // Bless to the formula at roll time, and counting it here
            // as well would roll the d4 twice.
            let line = sheet.save_line(code, &[]);
            let m = line.total + if vs_spell { line.conditional() } else { 0 };
            return Resolved {
                label: format!(
                    "{} Save{}",
                    code.to_uppercase(),
                    if vs_spell { " vs Spell" } else { "" }
                ),
                formula: d20_formula(m, mode),
                modifier: m,
                key: format!("{}_save", code),
                attack: None,
            };
        }
    }

    // Skill, by key ("ins") or by name ("insight")
    if let Some(s) = sheet.find_skill(&t) {
        // The grant is inside `skill_modifier` - applying it again
        // here would double every enchanted skill.
        let m = sheet.skill_modifier(&s.key);
        return Resolved {
            label: format!("{} ({})", s.name, s.ability.to_uppercase()),
            formula: d20_formula(m, mode),
            modifier: m,
            key: s.key.clone(),
            attack: None,
        };
    }

    // Ability check: "wis" or "wisdom check"
    let stem = t.strip_suffix(" check").unwrap_or(&t);
    if let Some(code) = ability_code(stem) {
        let m = sheet.ability_mod(code);
        return Resolved {
            label: format!("{} Check", code.to_uppercase()),
            formula: d20_formula(m, mode),
            modifier: m,
            key: format!("{}_check", code),
            attack: None,
        };
    }

    // Anything else is a raw formula. The dice engine will reject it if
    // it is nonsense, which is the right place for that to happen.
    Resolved {
        label: request.trim().to_string(),
        formula: t,
        modifier: 0,
        key: "custom".to_string(),
        attack: None,
    }
}

/* ============================ LOADING ============================ */

fn as_i64(v: &Value, key: &str, default: i64) -> i64 {
    v.get(key).and_then(|x| x.as_i64()).unwrap_or(default)
}

fn as_str(v: &Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

/// A Postgres text[] arrives as a JSON array. Absent reads as empty,
/// which is the right default for a proficiency list: claiming none is
/// safe, claiming one that was not granted is not.
/// A text column that may be absent or empty.
///
/// EMPTY IS ABSENT for these. A character sheet's free-text fields come
/// back "" from a form somebody tabbed through, and an empty hair
/// colour is not a hair colour - rendering one would put a blank row on
/// the panel where a fact should be.
fn as_opt_str(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(|x| x.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn as_strings(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(|x| x.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str()).map(str::to_string).collect())
        .unwrap_or_default()
}

/// The character row itself. Everything downstream needs the game and
/// the proficiencies before it can do anything, and a screen that wants
/// only the inventory should not have to buy the pack and the skill
/// catalogue to get them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub character_id: String,
    /// This character AS A HOLDER - 031. What their objects point at,
    /// and the thing every inventory read filters on now.
    pub entity_id: String,
    /// Where they are standing - 035. NULL is a real answer and the
    /// default: a character between scenes is not misfiled. What a drop
    /// needs, so a thing put down lands on a floor rather than in limbo.
    pub location_id: Option<String>,
    /// NULL means derive from level. A statblock states one; a player
    /// character does not have one to state. See 022.
    pub prof_bonus: Option<i64>,
    pub is_npc: bool,
    pub game_id: String,
    pub name: String,
    pub level: i64,
    pub narrative_pack: String,
    pub weapon_profs: Vec<String>,
    pub armor_profs: Vec<String>,
    /// 075.
    pub tool_profs: Vec<String>,
    /// 056. None for every character made before it, and for monsters.
    pub species_key: Option<String>,
    /// 122. This creature's own type, overriding what its species or
    /// statblock would say. None for almost everybody - see creature.rs.
    pub creature_type: Option<String>,
    /// 121. Which statblock this creature came from, and None for every
    /// player character. A KEY AND NOT A COPY, exactly as `species_key`
    /// is - the statblock's traits are read when the sheet loads, so a
    /// statblock corrected is every creature made from it corrected.
    pub npc_key: Option<String>,
    /// 058.
    pub body: Body,
    pub vitals: Vitals,
}

/// WHAT THIS PERSON LOOKS LIKE. 058.
///
/// The species carries a RANGE - the Unt'garoth run 7 to 10 feet and
/// 400 to 900 pounds - and this carries the value. A screen can then
/// show one against the other and say when somebody is unusual for
/// their people, which a single number never could.
///
/// ALL OPTIONAL, and none of it is defaulted. An unstated height is
/// unstated; guessing the middle of the species band would put a fact
/// on the sheet that nobody decided.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Body {
    pub height_ft: Option<f64>,
    pub weight_lb: Option<f64>,
    pub hair: Option<String>,
    pub skin: Option<String>,
    pub eyes: Option<String>,
    /// Anything else worth seeing at a glance. Prose the engine will
    /// never read - 043's contract.
    pub description: Option<String>,
    /// Tongues this character has BEYOND their people's. The panel
    /// shows the union and says which came from where.
    pub languages: Vec<crate::species::Tongue>,
}

/// What it takes to hit this character, and what it can take. 010.
///
/// `armor_class` is NOT in here, because it is not a stored fact - it is
/// computed from the loadout and lives on the Sheet, which is the only
/// place that knows what is worn.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Vitals {
    /// None means never set: a character that cannot yet be meaningfully
    /// attacked. Not zero, which would mean dead.
    pub hp_max: Option<i64>,
    pub hp_temp: i64,
    pub hp_temp_max: i64,
    /// `default` computes AC from what is worn; `flat` takes the
    /// override. See equipment::AcMode.
    pub ac_mode: String,
    /// The flat AC, and ONLY under ac_mode 'flat'. The export carries 14
    /// for a character whose real AC is 15 - see 010's header.
    pub ac_override: Option<i64>,
    pub death_successes: i64,
    pub death_failures: i64,
    pub exhaustion: i64,
    pub inspiration: bool,
    pub size: Option<String>,
}

pub fn load_profile(token: &str, character_id: &str) -> Result<Profile, String> {
    let chars = supabase::rest_get(
        token,
        "characters",
        &[
            (
                "select",
                "id,entity_id,location_id,game_id,name,level,prof_bonus,narrative_pack,\
                 weapon_profs,armor_profs,tool_profs,hp_max,hp_temp,hp_temp_max,\
                 ac_mode,ac_override,death_successes,death_failures,\
                 exhaustion,inspiration,size,species_key,npc_key,creature_type,\
                 height_ft,weight_lb,hair,skin,eyes,description,languages",
            ),
            ("id", &format!("eq.{}", character_id)),
        ],
    )?;
    let c = chars
        .as_array()
        .and_then(|a| a.first())
        .ok_or_else(|| "character not found, or not visible to you".to_string())?;

    // An empty pack column reads as base rather than as a pack with no
    // lines, so a row written before 006 added the default still narrates.
    let mut narrative_pack = as_str(c, "narrative_pack");
    if narrative_pack.trim().is_empty() {
        narrative_pack = narrative::BASE_PACK.to_string();
    }

    Ok(Profile {
        character_id: as_str(c, "id"),
        entity_id: as_str(c, "entity_id"),
        location_id: as_opt_str_char(c, "location_id"),
        prof_bonus: c.get("prof_bonus").and_then(|x| x.as_i64()),
        is_npc: c.get("is_npc").and_then(|x| x.as_bool()).unwrap_or(false),
        game_id: as_str(c, "game_id"),
        name: as_str(c, "name"),
        level: as_i64(c, "level", 1),
        narrative_pack,
        weapon_profs: as_strings(c, "weapon_profs"),
        armor_profs: as_strings(c, "armor_profs"),
        tool_profs: as_strings(c, "tool_profs"),
        species_key: c
            .get("species_key")
            .and_then(|x| x.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        creature_type: c
            .get("creature_type")
            .and_then(|x| x.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        npc_key: c
            .get("npc_key")
            .and_then(|x| x.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        body: Body {
            // numeric, so through the one reader that knows how
            // Postgres sends it - see supabase::numeric.
            height_ft: supabase::numeric_at(c, "height_ft"),
            weight_lb: supabase::numeric_at(c, "weight_lb"),
            hair: as_opt_str(c, "hair"),
            skin: as_opt_str(c, "skin"),
            eyes: as_opt_str(c, "eyes"),
            description: as_opt_str(c, "description"),
            languages: crate::species::tongues_of(c, "languages"),
        },
        vitals: Vitals {
            // Absent rather than defaulted: a character with no hp_max
            // has never had one set, which is not the same as being on
            // zero hit points.
            hp_max: c.get("hp_max").and_then(|x| x.as_i64()),
            hp_temp: as_i64(c, "hp_temp", 0),
            hp_temp_max: as_i64(c, "hp_temp_max", 0),
            ac_mode: {
                let m = as_str(c, "ac_mode");
                if m.trim().is_empty() { "default".to_string() } else { m }
            },
            ac_override: c.get("ac_override").and_then(|x| x.as_i64()),
            death_successes: as_i64(c, "death_successes", 0),
            death_failures: as_i64(c, "death_failures", 0),
            exhaustion: as_i64(c, "exhaustion", 0),
            inspiration: c
                .get("inspiration")
                .and_then(|x| x.as_bool())
                .unwrap_or(false),
            size: as_opt_str_char(c, "size"),
        },
    })
}

/// A column that is genuinely absent rather than empty. `size` is NULL
/// for every character that predates 010, and "" would be a size.
fn as_opt_str_char(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(|x| x.as_str())
        .map(str::to_string)
        .filter(|s| !s.trim().is_empty())
}

/// The ability modifier, before a Sheet exists to ask.
///
/// `Sheet::ability_mod` is the same arithmetic against the same map, but
/// AC has to be computed while the Sheet is still being assembled. One
/// expression in two places would drift, so the method delegates here.
/* ============================ SPECIES ============================ */

/// This game's version of a species, or the global one.
///
/// LIVES HERE RATHER THAN IN species.rs because that file is rules and
/// this one already talks to the database - the same line commands/mod.rs
/// draws, one level down. Callers outside the sheet need it too: an
/// encumbrance read has to know a people carries one size larger.
/// What a statblock says this creature IS.
///
/// 121. THE MONSTER HALF OF 116. Resistance had four sources and not
/// one of them was a statblock, which is where 5e keeps most of its -
/// half the Monster Manual resists something and `npcs` had nowhere to
/// say it.
///
/// READ, NOT COPIED, which is `load_species` just below doing the same
/// thing for the same reason: correcting a statblock corrects every
/// goblin already on the board, with no backfill and nothing touched
/// mid-fight.
///
/// A FAILURE IS NO TRAITS RATHER THAN NO SHEET. A statblock deleted out
/// from under a creature leaves it resisting nothing, which is what it
/// had before 121 - better than a character screen that will not open.
pub(crate) fn load_npc_traits(
    token: &str,
    game_id: &str,
    key: &str,
) -> NpcTraits {
    if key.is_empty() {
        return NpcTraits::default();
    }
    let Ok(rows) = crate::supabase::rest_get(
        token,
        "npcs",
        &[
            ("select", "key,name,grants,creature_type,game_id"),
            ("key", &format!("eq.{}", key)),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
        ],
    ) else {
        return NpcTraits::default();
    };
    let empty = Vec::new();
    let rows = rows.as_array().unwrap_or(&empty);

    // THE GAME'S OWN ROW WINS over the global one, which is the
    // precedence every other catalogue uses - and the same ordering
    // `instantiate_npc` applies when it picks the statblock to build
    // from, so the traits come off the row the creature was made from.
    let row = rows
        .iter()
        .find(|r| r.get("game_id").map(|g| !g.is_null()).unwrap_or(false))
        .or_else(|| rows.first());

    let Some(r) = row else {
        return NpcTraits::default();
    };
    // THE STATBLOCK'S NAME AS THE FALLBACK SOURCE, so a grant that
    // forgot to name itself reads as "Goblin" on the sheet rather than
    // as a blank chip.
    let name = r.get("name").and_then(|v| v.as_str()).unwrap_or("its kind");
    NpcTraits {
        grants: match r.get("grants") {
            Some(g) => crate::grants::parse(g, name),
            None => Vec::new(),
        },
        creature_type: r
            .get("creature_type")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string),
    }
}

/// What a statblock says about a creature made from it.
///
/// 122. ONE READ FOR BOTH, because they come off the same row. 121
/// fetched it for the grants alone and adding a second request for the
/// type beside it would be two round trips for one row.
#[derive(Debug, Default, Clone)]
pub(crate) struct NpcTraits {
    pub grants: Vec<crate::grants::Grant>,
    /// 5e's fourteen. A monster has no species row to ask, so the
    /// statblock states its own.
    pub creature_type: Option<String>,
}

pub(crate) fn load_species(
    token: &str,
    game_id: &str,
    key: &str,
) -> Result<Option<crate::species::Species>, String> {
    if key.is_empty() {
        return Ok(None);
    }
    Ok(load_species_map(token, game_id, &[key.to_string()])?.remove(key))
}

/// Several peoples in one request, for a screen that reads a roster
/// rather than a character.
///
/// THE SINGLE READ IS THIS ONE WITH A LIST OF ONE. An encounter holds
/// six creatures of maybe two peoples, and a species read per creature
/// is the shape `batch_character_stats` exists to avoid - it already
/// does the characters, the abilities, the objects and the catalogue
/// four requests rather than four per head.
pub(crate) fn load_species_map(
    token: &str,
    game_id: &str,
    keys: &[String],
) -> Result<HashMap<String, crate::species::Species>, String> {
    // QUOTED, like every other `in.(...)` list in the crate. A species
    // key is a slug today and quoting it costs nothing; an unquoted
    // list is one key with a comma in it away from asking for two
    // species that do not exist - see the trap supabase.rs records
    // about interpolating into a filter.
    let wanted: Vec<String> = keys
        .iter()
        .filter(|k| !k.is_empty())
        .cloned()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .map(|k| crate::narrative::quoted(&k))
        .collect();
    if wanted.is_empty() {
        return Ok(HashMap::new());
    }
    // NO GAME MEANS THE GLOBAL ROWS, not a broken filter. A caller can
    // arrive without one - `roll_for` is handed an actor id and reads
    // the game off its encounter, which RLS can refuse - and
    // `game_id.eq.` with nothing after it is a 400 that would take the
    // initiative roll down with it. The global species is the right
    // answer there; only a game's own override is lost.
    let scope = if game_id.is_empty() {
        "(game_id.is.null)".to_string()
    } else {
        format!("(game_id.is.null,game_id.eq.{})", game_id)
    };
    let rows = supabase::rest_get(
        token,
        "species",
        &[
            ("select", crate::species::SPECIES_COLUMNS),
            ("key", &format!("in.({})", wanted.join(","))),
            ("or", &scope),
        ],
    )?;
    Ok(crate::species::collapse(rows.as_array().unwrap_or(&Vec::new()))
        .into_iter()
        .map(|sp| (sp.key.clone(), sp))
        .collect())
}

/// What this character can cast right now, for the roll box.
///
/// 154. PREPARED AND CANTRIPS, NEVER THE BOOK. 107's three states say
/// it: a cantrip is known, a prepared spell is up today, and a spell in
/// the book is held and not available. Rolling one out of the book
/// would be casting something the character has not prepared.
///
/// THE SAME TENANCY DANCE `load_species_map` DOES, and for the same
/// reason - a game may override a catalogue spell, the game's row wins,
/// and an empty `game_id` asks for the global rows rather than sending
/// a filter that 400s.
fn load_castable(
    token: &str,
    game_id: &str,
    character_id: &str,
) -> Result<Vec<crate::spellcast::Known>, String> {
    let chosen = supabase::rest_get(
        token,
        "character_spells",
        &[
            ("select", "spell_key,state"),
            ("character_id", &format!("eq.{}", character_id)),
            ("state", "in.(prepared,cantrip)"),
        ],
    )?;
    let keys: Vec<String> = chosen
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|r| r.get("spell_key")?.as_str())
        .filter(|k| !k.is_empty())
        .map(crate::narrative::quoted)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    if keys.is_empty() {
        return Ok(Vec::new());
    }

    let scope = if game_id.is_empty() {
        "(game_id.is.null)".to_string()
    } else {
        format!("(game_id.is.null,game_id.eq.{})", game_id)
    };
    let rows = supabase::rest_get(
        token,
        "spells",
        &[
            (
                "select",
                "key,name,roll_name,level,cast_type,casting_time,save_ability,dice,range,game_id",
            ),
            ("key", &format!("in.({})", keys.join(","))),
            ("or", &scope),
        ],
    )?;

    // THE GAME'S OWN ROW WINS, and a key appears at most once. Two rows
    // with the same key is the normal case rather than an error - that
    // is what an override IS - and leaving both in would give `find` two
    // answers and let it return whichever the database happened to list
    // first.
    let mut out: HashMap<String, crate::spellcast::Known> = HashMap::new();
    for r in rows.as_array().unwrap_or(&Vec::new()) {
        let key = as_str(r, "key");
        if key.is_empty() {
            continue;
        }
        let mine = r.get("game_id").map(|g| !g.is_null()).unwrap_or(false);
        if out.contains_key(&key) && !mine {
            continue;
        }
        let name = as_str(r, "name");
        out.insert(
            key.clone(),
            crate::spellcast::Known {
                roll_name: as_opt_str(r, "roll_name").unwrap_or_else(|| name.clone()),
                key,
                name,
                level: as_i64(r, "level", 0),
                cast_type: as_str(r, "cast_type"),
                casting_time: as_opt_str(r, "casting_time"),
                save_ability: as_opt_str(r, "save_ability"),
                dice: as_opt_str(r, "dice"),
                range: as_opt_str(r, "range"),
            },
        );
    }
    Ok(out.into_values().collect())
}

/// EFFECTIVE ABILITY SCORES FOR A SET OF CHARACTERS, with each one's
/// people already applied - and the peoples themselves, because the
/// unarmoured floor needs the species as well as the modifier.
///
/// WHY THIS EXISTS. Four separate places read `character_abilities`
/// and turned a score into a modifier: the target list, the initiative
/// roll, the level button and the hit-point rederivation. Every one of
/// them read the STORED number, which is what somebody rolled and not
/// what the character has - 056 keeps those apart on purpose. The
/// target list is where it showed first and worst, reading Garn at AC
/// 10 while his sheet said 14.
///
/// One loader, so the next screen that needs a modifier cannot get a
/// different answer than the sheet. `load_sheet` does not use it only
/// because it is already reading everything for one character; it
/// applies the same `apply_species` to get there.
pub(crate) struct Effective {
    scores: HashMap<String, HashMap<String, Ability>>,
    peoples: HashMap<String, crate::species::Species>,
    /// How many swings the Attack action buys, per character - 061.
    /// Derived from their class and level, which the first request
    /// already had to read.
    attacks: HashMap<String, i64>,
}

impl Effective {
    /// Built from rows a test has in hand rather than from the
    /// network, so the defaults every call site leans on are pinned.
    #[cfg(test)]
    pub(crate) fn of(
        scores: HashMap<String, HashMap<String, Ability>>,
        peoples: HashMap<String, crate::species::Species>,
    ) -> Self {
        Effective { scores, peoples, attacks: HashMap::new() }
    }

    /// How many attacks this character's Attack action buys.
    ///
    /// ONE FOR ANYBODY WITHOUT A CLASS, which is every monster in the
    /// game. A statblock's multiattack is a different mechanism that
    /// nothing reads yet, and claiming a goblin gets one swing is both
    /// true today and the safe way to be wrong.
    pub(crate) fn attacks(&self, character_id: &str) -> i64 {
        self.attacks.get(character_id).copied().unwrap_or(1)
    }

    /// The SCORE itself, species bonus included, for the one rule that
    /// wants the number rather than the modifier: carrying capacity is
    /// Strength times fifteen, not Strength's +2.
    ///
    /// Ten where there is no row, which is what every reader of a
    /// missing ability has always assumed - the schema seeds all six at
    /// creation, so an absent row is a fault somewhere else and the
    /// average is the harmless reading of it.
    pub(crate) fn score(&self, character_id: &str, code: &str) -> i64 {
        self.scores
            .get(character_id)
            .and_then(|a| a.get(code))
            .map(|a| a.score)
            .unwrap_or(10)
    }

    /// This character's people, for a rule that needs the row itself
    /// rather than a number off it.
    ///
    /// THIS EXISTED, WAS DELETED AS DEAD CODE, AND WAS WANTED THE NEXT
    /// DAY. Carrying size is the caller: an Unt'garoth counts as one
    /// size larger for what they can lift, which is a fact about the
    /// people and not about any ability. Without it the carry path read
    /// the species a second time and resolved Strength by hand -
    /// correctly, and as the last copy of the rule this loader exists
    /// to own.
    pub(crate) fn people(&self, character_id: &str) -> Option<&crate::species::Species> {
        self.peoples.get(character_id)
    }

    /// The modifier, species bonus included. Zero for a character with
    /// no rows, which is what every one of the four sites defaulted to
    /// and is the harmless reading - the schema seeds all six on
    /// creation, so an absent row is a fault elsewhere.
    pub(crate) fn modifier(&self, character_id: &str, code: &str) -> i64 {
        match self.scores.get(character_id) {
            Some(a) => ability_mod_of(a, code),
            None => 0,
        }
    }

    /// Their species' unarmoured floor, resolved against their own
    /// scores. None for anyone whose people does not state one.
    pub(crate) fn unarmored(&self, character_id: &str) -> Option<(i64, i64)> {
        let abilities = self.scores.get(character_id)?;
        unarmored_rule(self.peoples.get(character_id), abilities)
    }
}

/// Four requests for any number of characters: who they are, what they
/// rolled, what their peoples add, and what their classes buy.
///
/// THE FOURTH CAME FREE OF A FIFTH. The first request already reads
/// `characters`, so `class_key` and `level` cost nothing extra there;
/// only the class catalogue itself is a new round trip, and it is
/// twelve rows.
pub(crate) fn load_effective(
    token: &str,
    game_id: &str,
    character_ids: &[String],
) -> Result<Effective, String> {
    let ids: Vec<String> = character_ids
        .iter()
        .filter(|s| !s.is_empty())
        .cloned()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    if ids.is_empty() {
        return Ok(Effective {
            scores: HashMap::new(),
            peoples: HashMap::new(),
            attacks: HashMap::new(),
        });
    }
    let list = ids.join(",");

    let chars = supabase::rest_get(
        token,
        "characters",
        &[
            ("select", "id,species_key"),
            ("id", &format!("in.({})", list)),
        ],
    )?;
    let mut key_of: HashMap<String, String> = HashMap::new();
    for r in chars.as_array().unwrap_or(&Vec::new()) {
        let Some(id) = r.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        if let Some(k) = r.get("species_key").and_then(|v| v.as_str()) {
            if !k.is_empty() {
                key_of.insert(id.to_string(), k.to_string());
            }
        }
    }

    // 073. WHICH CLASSES, AND AT WHAT LEVEL, FOR EVERYBODY AT ONCE.
    // This used to come free off the `characters` row - one class, one
    // level, no extra request - and a character who is two things has
    // neither. One `in.()` over the whole roster is the price, and it
    // is one round trip however many people are being asked about.
    let class_rows = supabase::rest_get(
        token,
        "character_classes",
        &[
            ("select", "character_id,class_key,level"),
            ("character_id", &format!("in.({})", list)),
            ("order", "added_at.asc,class_key.asc"),
        ],
    )?;
    // character -> (class key, level), in the order taken.
    let mut classed: HashMap<String, Vec<(String, i64)>> = HashMap::new();
    for r in class_rows.as_array().unwrap_or(&Vec::new()) {
        let Some(cid) = r.get("character_id").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(key) = r.get("class_key").and_then(|v| v.as_str()) else {
            continue;
        };
        let level = r.get("level").and_then(|v| v.as_i64()).unwrap_or(1);
        classed
            .entry(cid.to_string())
            .or_default()
            .push((key.to_string(), level));
    }

    let rows = supabase::rest_get(
        token,
        "character_abilities",
        &[
            ("select", "character_id,ability,score,save_prof"),
            ("character_id", &format!("in.({})", list)),
        ],
    )?;

    let catalogue = load_species_map(
        token,
        game_id,
        &key_of.values().cloned().collect::<Vec<_>>(),
    )?;

    let mut scores: HashMap<String, HashMap<String, Ability>> = HashMap::new();
    for r in rows.as_array().unwrap_or(&Vec::new()) {
        let Some(cid) = r.get("character_id").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(code) = r.get("ability").and_then(|v| v.as_str()) else {
            continue;
        };
        let score = r.get("score").and_then(|v| v.as_i64()).unwrap_or(10);
        let prof = r.get("save_prof").and_then(|v| v.as_bool()).unwrap_or(false);
        scores
            .entry(cid.to_string())
            .or_default()
            .insert(code.to_string(), Ability::plain(score, prof));
    }

    // THE FOURTH REQUEST, SKIPPED WHEN NOBODY HAS A CLASS - which is
    // the common case for a roster of monsters, and the reason this is
    // not folded into the species read.
    let mut attacks: HashMap<String, i64> = HashMap::new();
    if !classed.is_empty() {
        let keys: Vec<String> = classed
            .values()
            .flatten()
            .map(|(k, _)| k.clone())
            .collect();
        let catalogue = crate::class::load_map(token, game_id, &keys)?;
        for (cid, held) in &classed {
            // THE BEST OF THEM, NOT THE SUM - 5e is explicit that Extra
            // Attack from two classes does not add up, and
            // multiclass::attacks is where that is written and tested.
            //
            // A class_key pointing at nothing contributes nothing
            // rather than refusing the whole read. 055 cannot use a
            // foreign key to prevent it, and a dangling reference
            // should not empty a screen.
            let each: Vec<i64> = held
                .iter()
                .filter_map(|(key, level)| {
                    catalogue.iter().find(|c| &c.key == key).map(|c| c.attacks_at(*level))
                })
                .collect();
            attacks.insert(cid.clone(), crate::multiclass::attacks(&each));
        }
    }

    // 091. WHAT THEY CHOSE, FOR EVERYONE AT ONCE. The sheet applies
    // Ability Score Improvements and so must this - otherwise the
    // target list, the initiative roll and the hit-point rederivation
    // would all read a Strength the sheet disagrees with, which is
    // exactly the fault 056's one-loader note was written about.
    let per_character: Vec<(String, Vec<crate::multiclass::Taken>)> = classed
        .iter()
        .map(|(cid, held)| {
            (
                cid.clone(),
                held.iter()
                    .map(|(key, level)| crate::multiclass::Taken {
                        key: key.clone(),
                        level: *level,
                        // UNREAD HERE. `features::held` compares levels
                        // and never asks what a class rolls for hit
                        // points, and inventing a die to fill a field
                        // would be the lie this codebase keeps warning
                        // about.
                        hit_die: 0,
                    })
                    .collect(),
            )
        })
        .collect();
    let bumps = crate::features::load_ability_sources(token, game_id, &per_character)?;

    let mut peoples: HashMap<String, crate::species::Species> = HashMap::new();
    for (cid, key) in &key_of {
        let Some(sp) = catalogue.get(key) else {
            continue;
        };
        if let Some(abilities) = scores.get_mut(cid) {
            apply_species(abilities, sp);
        }
        peoples.insert(cid.clone(), sp.clone());
    }

    // 091. AFTER THE SPECIES AND OVER EVERYBODY, exactly as load_sheet
    // does it in the same order. A SEPARATE LOOP because the species
    // loop walks only characters who HAVE a species - a classed
    // character with none would have had their Ability Score
    // Improvement silently dropped, which is the quiet half-application
    // this whole change exists to end.
    for (cid, mine) in &bumps {
        if let Some(abilities) = scores.get_mut(cid) {
            apply_bumps(abilities, mine, peoples.get(cid));
        }
    }

    Ok(Effective { scores, peoples, attacks })
}

/// A species' unarmoured floor, with its ability already resolved to a
/// modifier: Unyielding Defense is 12 + CON rather than 10 + DEX.
///
/// ONE PLACE, BECAUSE TWO SCREENS ASK. The sheet computes what it takes
/// to hit you; the encounter's target list computes what it takes to
/// hit you, and for a while they disagreed - the roster passed None
/// here and read every unarmoured character at 10 + DEX. Garn's sheet
/// said AC 14 and the list the goblins actually roll against said 10.
///
/// Resolved in this file rather than in equipment.rs for the reason
/// given at the call site: equipment does not know what an ability is.
pub(crate) fn unarmored_rule(
    species: Option<&crate::species::Species>,
    abilities: &HashMap<String, Ability>,
) -> Option<(i64, i64)> {
    let sp = species?;
    let base = sp.unarmored_ac_base?;
    let ability = sp.unarmored_ac_ability.as_deref()?;
    Some((base, ability_mod_of(abilities, ability)))
}

/// Raise every score by what the species adds, held to its ceiling.
///
/// ONE PLACE, AFTER THE READ. Every modifier in the app comes from
/// `Ability::score` through `ability_mod_of`, so applying the bonus
/// here means skills, saves, attack rolls, carrying and armour class
/// all pick it up without any of them knowing species exist. The
/// alternative - each site adding the bonus itself - is the two-places
/// problem that 049 and supabase::numeric were both about.
pub(crate) fn apply_species(
    abilities: &mut HashMap<String, Ability>,
    sp: &crate::species::Species,
) {
    for (code, a) in abilities.iter_mut() {
        let bonus = sp.bonus_for(code);
        if bonus == 0 {
            continue;
        }
        a.bonus = bonus;
        // 080. NAMED, so the sheet can say "Unt'garoth +2" rather than
        // an unattributed "+2" the reader has to take on faith.
        a.sources.push(AbilitySource { name: sp.name.clone(), value: bonus });
        a.score = crate::species::effective_score(a.base, bonus, sp.maximum_for(code));
    }
}

/// Add what the chosen class features give, named.
///
/// 091. AFTER THE SPECIES AND NEVER INSTEAD OF IT. A character has both
/// - Falon is an Unt'garoth with a Fighter's Ability Score Improvement
/// - and the point of `sources` is that each says where it came from
/// rather than arriving as one unexplained number.
///
/// THE CEILING IS THE SPECIES' OR TWENTY. 5e caps an improvement at 20
/// and a species that states a higher maximum raises it - the
/// Unt'garoth's Strength goes to 21. `effective_score` already applies
/// the species' own maximum; this applies the same ceiling to what the
/// improvements add, so the two cannot disagree about where a score
/// stops.
/// Every ability grant the loadout is carrying, applied on top of a
/// finished natural score. 100.
///
/// NOT CAPPED, and that is the difference between this and
/// `apply_bumps`. A ceiling is about what training can reach; an item
/// is the stated exception to it, which is the rule Dave gave in the
/// same breath as the cap itself.
///
/// A SET SETTLES BEFORE THE ADDS LAND, which `grants::apply` owns -
/// so Gauntlets of Ogre Power and a +2 belt come to the same total on
/// a wizard and a barbarian, which is what each item claims alone.
///
/// EVERY CONTRIBUTION IS NAMED. `Ability::sources` has carried a name
/// per bonus since 056 and its own comment said "and later an
/// item's"; this is later. A player asking why their Strength reads
/// 21 gets the species, the improvement and the gauntlets listed
/// rather than a number to take on faith.
/// Every live grant on everything worn, in one list.
///
/// ONE PLACE TO FLATTEN IT, because three consumers want the same pile
/// and the fourth will too. `Owned::grants` is already filtered to
/// what is live - in a slot, attuned where the grant asks - so this is
/// only the gathering.
pub(crate) fn worn_grants(loadout: &[equipment::Owned]) -> Vec<crate::grants::Grant> {
    loadout.iter().flat_map(|o| o.grants.iter().cloned()).collect()
}

pub(crate) fn apply_grants(
    abilities: &mut HashMap<String, Ability>,
    loadout: &[equipment::Owned],
) {
    let all = worn_grants(loadout);
    if all.is_empty() {
        return;
    }
    for (code, a) in abilities.iter_mut() {
        let after = crate::grants::apply(a.score, &all, code);
        if after == a.score {
            continue;
        }
        for g in crate::grants::sources_for(&all, code) {
            a.sources.push(AbilitySource {
                name: g.source.clone(),
                value: g.value,
            });
        }
        a.score = after;
        a.bonus = a.score - a.base;
    }
}

pub(crate) fn apply_bumps(
    abilities: &mut HashMap<String, Ability>,
    bumps: &[(String, String, i64)],
    species: Option<&crate::species::Species>,
) {
    for (code, name, value) in bumps {
        let Some(a) = abilities.get_mut(code) else {
            continue;
        };
        // `maximum_for` already falls back to 20, so a character with
        // no species gets the same ceiling without a second default
        // written here.
        let ceiling = species
            .map(|sp| sp.maximum_for(code))
            .unwrap_or(crate::species::DEFAULT_MAXIMUM);
        a.score = (a.score + value).min(ceiling);
        a.bonus = a.score - a.base;
        a.sources.push(AbilitySource { name: name.clone(), value: *value });
    }
}

pub(crate) fn ability_mod_of(abilities: &HashMap<String, Ability>, code: &str) -> i64 {
    match abilities.get(code) {
        Some(a) => (a.score - 10).div_euclid(2),
        None => 0,
    }
}

/// Read everything resolve_request needs, plus the narrative pack and
/// the equipped loadout, in seven queries.
///
/// Seven is more than this wants to be. It runs on every roll, and the
/// two equipment reads sit ahead of the dice with the pack read. None of
/// them is slow individually and none has been worth splitting yet — but
/// this is the count to watch, and `preview_request` pays all of it for
/// a hover.
///
/// Could be one query with PostgREST embedding, but four explicit reads
/// are easier to debug when a policy denies one of them — an embedded
/// join that silently returns fewer rows looks like missing data rather
/// than a permission problem.
pub fn load_sheet(token: &str, character_id: &str) -> Result<Sheet, String> {
    let profile = load_profile(token, character_id)?;
    let game_id = profile.game_id.clone();

    let abil_rows = supabase::rest_get(
        token,
        "character_abilities",
        &[
            ("select", "ability,score,save_prof"),
            ("character_id", &format!("eq.{}", character_id)),
        ],
    )?;
    let mut abilities = HashMap::new();
    if let Some(rows) = abil_rows.as_array() {
        for r in rows {
            abilities.insert(
                as_str(r, "ability"),
                // Base for now; `apply_species` raises the effective
                // score once the people are known. One place, after the
                // read, rather than threading a species through it.
                Ability::plain(
                    as_i64(r, "score", 10),
                    r.get("save_prof").and_then(|x| x.as_bool()).unwrap_or(false),
                ),
            );
        }
    }

    // Global rows plus this game's overrides, in one request. The
    // override wins because it is inserted second below.
    let skill_rows = supabase::rest_get(
        token,
        "skills",
        &[
            ("select", "key,name,ability,game_id,sort_order"),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
            ("order", "sort_order.asc"),
        ],
    )?;

    let mut by_key: HashMap<String, SkillDef> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    if let Some(rows) = skill_rows.as_array() {
        // Globals first, then overrides, so an override replaces the
        // global entry it shadows rather than appearing beside it.
        for pass in [true, false] {
            for r in rows {
                let is_global = r.get("game_id").map(|g| g.is_null()).unwrap_or(true);
                if is_global != pass {
                    continue;
                }
                let key = as_str(r, "key");
                if !by_key.contains_key(&key) {
                    order.push(key.clone());
                }
                by_key.insert(
                    key.clone(),
                    SkillDef {
                        key,
                        name: as_str(r, "name"),
                        ability: as_str(r, "ability"),
                    },
                );
            }
        }
    }
    let skills: Vec<SkillDef> = order
        .iter()
        .filter_map(|k| by_key.get(k).cloned())
        .collect();

    let prof_rows = supabase::rest_get(
        token,
        "character_skills",
        &[
            ("select", "skill_key,prof"),
            ("character_id", &format!("eq.{}", character_id)),
        ],
    )?;
    let mut profs = HashMap::new();
    if let Some(rows) = prof_rows.as_array() {
        for r in rows {
            // A half-proficiency is 0.5, so this is genuinely numeric
            // rather than an integer. The comment that used to sit here
            // had the representation backwards - see supabase::numeric.
            let p = supabase::numeric_at(r, "prof").unwrap_or(0.0);
            profs.insert(as_str(r, "skill_key"), p);
        }
    }

    // BEFORE THE LOADOUT AND BEFORE AC, because it moves the scores
    // both of those read. Applied straight onto the abilities map, so
    // every modifier downstream carries it without knowing why.
    // READ BEFORE THE ABILITIES ARE FINISHED (091). An Ability
    // Score Improvement moves a score, and the armour class and the
    // loadout below both read one - so the classes have to be known
    // before `apply_bumps`, not after.
    //
    // LEAD FIRST, so the screen does not have to decide which class
    // that is. `load_taken` returns them in the order they were TAKEN,
    // which is what the hit-point rule reads; what a sheet shows first
    // is the one with the most levels in it, and both orders are
    // multiclass.rs's to know.
    let classes = crate::multiclass::lead_first(
        &crate::class::load_taken(token, &game_id, character_id)?,
    );

    // 154. WHAT THEY COULD CAST, and only if they cast at all.
    //
    // TWO MORE QUERIES, PAID BY CLERICS AND NOBODY ELSE. The note on
    // this function says seven is the count to watch and that a hover
    // preview pays all of it; this makes it nine for a caster and
    // leaves it at seven for every fighter, every monster and every
    // creature in the bestiary. `casting::caster` is the gate and it is
    // already computed from `classes`, which is read above.
    //
    // TWO AND NOT ONE because `character_spells.spell_key` references
    // `spells` BY VALUE - the nullable-tenancy pattern, where a partial
    // unique index cannot back a foreign key - so there is no
    // relationship for PostgREST to embed across and the keys have to
    // be known before the catalogue can be asked about them.
    let caster = crate::casting::caster(&classes);
    let spells = match &caster {
        Some(_) => load_castable(token, &game_id, character_id)?,
        None => Vec::new(),
    };

    let species = match profile.species_key.as_deref() {
        Some(key) => load_species(token, &game_id, key)?,
        None => None,
    };
    if let Some(sp) = &species {
        apply_species(&mut abilities, sp);
    }
    // 091. AND WHAT THEY CHOSE. An Ability Score Improvement was
    // recorded by 087 and applied to nothing until this line; a feature
    // that stores a decision and moves no number is half a feature.
    let bumps = crate::features::load_ability_sources(
        token,
        &game_id,
        &[(character_id.to_string(), classes.clone())],
    )?;
    if let Some(mine) = bumps.get(character_id) {
        apply_bumps(&mut abilities, mine, species.as_ref());
    }

    // 073. One request, and skipped entirely for anybody with no class
    // rows - see class::load_taken, which returns early on an empty
    // read rather than asking the catalogue about nothing.
    // 076. The catalogue for whatever classes they hold, so Karma can
    // ask which skills this class sums. Costs nothing for the
    // classless - `load_map` returns early on an empty key list, which
    // is every monster in the game.
    let class_catalogue = crate::class::load_map(
        token,
        &game_id,
        &classes.iter().map(|t| t.key.clone()).collect::<Vec<_>>(),
    )?;

    let line_rows = narrative::load_lines(token, &game_id, &profile.narrative_pack)?;
    let narratives = narrative::resolve_lines(&line_rows, &profile.narrative_pack);

    let loadout = equipment::load_loadout(
        token,
        &profile.entity_id,
        &game_id,
        &profile.weapon_profs,
        &profile.armor_profs,
        &profile.tool_profs,
        true,
    )?;

    // 100. WHAT THE MAGIC ADDS, AFTER EVERYTHING NATURAL.
    //
    // THE ORDER IS THE RULE AND IT IS THE WHOLE POINT. A rolled score,
    // then the people's bonus, then improvements - all three bound by
    // 099's ceiling, because that ceiling is about what a body can be
    // trained to. Then this, which is not bound by it: Dave's rule is
    // that a spell or an item MAY carry somebody past their cap, and
    // Gauntlets of Ogre Power would be pointless on a Ny'ook
    // otherwise.
    //
    // EVERY MODIFIER IN THE APP COMES OFF `Ability::score`, so putting
    // it here is what makes a Headband of Intellect reach Arcana, an
    // INT save and the carrying rules without any of them being told
    // about items.
    apply_grants(&mut abilities, &loadout);

    // Computed here rather than stored, from the loadout that was just
    // read. DEX is the live modifier, so a stat change moves AC the same
    // turn rather than at the next reseed.
    // Only the weapons: armour and gear have no techniques, and asking
    // for their keys would widen the query for nothing.
    // (object, type) PAIRS, not just the keys. 050 lets one sword
    // disagree with another of the same kind about what it can do, and
    // a list of keys cannot tell them apart - see expand_for_objects.
    let weapons: Vec<(String, String)> = loadout
        .iter()
        .filter(|o| o.item.kind == "weapon")
        .map(|o| (o.id.clone(), o.item.key.clone()))
        .collect();
    let techniques = crate::attack::load_for_loadout(token, &game_id, &weapons)?;

    let worn: Vec<&equipment::Item> = loadout.iter().map(|o| &o.item).collect();
    // A species' unarmoured rule, with its ability already resolved to
    // a modifier - Unyielding Defense is 12 + CON rather than 10 + DEX.
    // Resolved here because this is where the abilities are; equipment
    // does not know what an ability is.
    let unarmored = unarmored_rule(species.as_ref(), &abilities);
    // 100. THE MAGIC GOES ON THE OUTSIDE, and `armor_class` never
    // learns that items can be enchanted. It answers what armour, a
    // shield and a species floor come to; a Ring of Protection is a
    // bonus on that answer and Barkskin ("your AC is 16") is a floor
    // under it - which is `grants::apply`, the same two rules in the
    // same order as every other target.
    let armor_class = crate::grants::apply(
        equipment::armor_class(
            ability_mod_of(&abilities, "dex"),
            &worn,
            equipment::AcMode::parse(&profile.vitals.ac_mode),
            profile.vitals.ac_override,
            unarmored,
        ),
        &worn_grants(&loadout),
        "ac",
    );

    // 116. EVERY REASON DAMAGE LANDS DIFFERENTLY ON THIS PERSON.
    //
    // FOUR SOURCES, ONE VOCABULARY. The species has its own column
    // from 056; the other three speak 100's grant language, which is
    // the whole reason resistance needed no table of its own.
    //
    // THE RUNNING SPELLS ARE READ LAST AND DO NOT STOP THE SHEET. An
    // effect is the only one of the four that needs the game clock, and
    // a sheet that refused to load because the effects table was slow
    // would be a worse failure than a missing resistance - the same
    // judgement `swing` makes about a Bless it cannot read.
    let mut resist_sources: Vec<crate::resist::Source> = Vec::new();
    if let Some(sp) = &species {
        resist_sources.extend(crate::resist::from_species(&sp.damage_resistances, &sp.name));
    }
    resist_sources.extend(crate::resist::from_grants(&worn_grants(&loadout)));
    // 121. AND WHAT ITS KIND IS. A monster's traits live on the
    // statblock, read through `npc_key` rather than copied onto the
    // creature - None for every player character, which is most of them.
    let from_statblock = match profile.npc_key.as_deref() {
        Some(key) => load_npc_traits(token, &game_id, key),
        None => NpcTraits::default(),
    };
    resist_sources.extend(crate::resist::from_grants(&from_statblock.grants));
    resist_sources.extend(crate::resist::from_grants(
        &crate::features::load_passive_grants(token, &game_id, &classes)?,
    ));
    // READ ONCE AND USED TWICE. The resistances want these and so do
    // the save lines - a Bless running on somebody is a save modifier
    // and belongs in the column beside the others.
    let live = crate::commands::effects::grants_on(token, &game_id, character_id)
        .unwrap_or_default();
    resist_sources.extend(crate::resist::from_grants(&live));
    let resistances = crate::resist::standing(&resist_sources);

    // 122. AND WHAT IT IS. The creature's own word, then its people's,
    // then - for a monster, which has no people - its statblock's.
    // `creature::of` owns the first two and this adds the third, which
    // only a creature with an npc_key can have.
    let creature_type = crate::creature::of(
        profile.creature_type.as_deref(),
        species.as_ref().and_then(|sp| sp.creature_type.as_deref()),
    )
    .or_else(|| {
        from_statblock
            .creature_type
            .as_deref()
            .and_then(crate::creature::Kind::parse)
    })
    .map(|k| k.as_str().to_string());

    // 133. WHETHER THIS IS A CREATURE, which decides whether a
    // technique gate means anything - read off the statblock link that
    // 121 already follows for traits.
    let from_statblock_row = profile.npc_key.is_some();

    let mut sheet = Sheet {
        // 150. OFF THE CLASS ROWS, the same ones a player character
        // has. A creature that casts has class levels - see
        // casting::caster, and 022 for why there is no second path.
        caster,
        spells,
        from_statblock: from_statblock_row,
        gate_level: crate::attack::gate_level(profile.level, from_statblock_row),
        location_id: profile.location_id,
        character_id: Some(profile.character_id),
        game_id,
        name: profile.name,
        level: profile.level,
        classes,
        // NULL for a player character, so `proficiency_bonus` falls back
        // to the level formula. An instance off a statblock carries the
        // stated value - see 022.
        prof_bonus: profile.prof_bonus,
        abilities,
        skills,
        profs,
        skill_mods: HashMap::new(),
        narrative_pack: profile.narrative_pack,
        narratives,
        weapon_profs: profile.weapon_profs,
        armor_profs: profile.armor_profs,
        tool_profs: profile.tool_profs,
        // SET BELOW, once the sheet exists - see the note at the end
        // of this function.
        karma: None,
        carry_size: species
            .as_ref()
            .map(|sp| sp.carry_size())
            .or_else(|| profile.vitals.size.clone()),
        species,
        body: profile.body,
        vitals: profile.vitals,
        armor_class,
        creature_type,
        resistances,
        // SET BELOW, off the finished sheet - a save needs the
        // abilities with the species bonus in them, the proficiency
        // bonus derived from the level and the loadout in place, which
        // is the same reason Karma and the skill lines are resolved
        // after this struct exists.
        save_lines: HashMap::new(),
        techniques,
        loadout,
    };
    // LAST, AND OFF THE FINISHED SHEET. Karma sums skill MODIFIERS, and
    // a skill modifier needs the abilities with the species bonus
    // already in them, the proficiency bonus derived from the level,
    // and the skill catalogue resolved for this game. All three are
    // facts the sheet has and none of them is a fact this function had
    // before it built one - so Karma is derived from the sheet rather
    // than assembled in parallel beside it.
    // AFTER THE SHEET EXISTS, because `skill_modifier` is a method on
    // it and needs the abilities, the proficiencies and the loadout
    // all in place - the same reason karma is resolved here.
    let keys: Vec<String> = sheet.skills.iter().map(|s| s.key.clone()).collect();
    let worn = worn_grants(&sheet.loadout);
    sheet.skill_mods = keys
        .into_iter()
        .map(|k| {
            let natural = sheet.skill_modifier_natural(&k);
            let line = SkillLine {
                natural,
                parts: crate::grants::breakdown(natural, &worn, &format!("skill.{}", k)),
                total: sheet.skill_modifier(&k),
            };
            (k, line)
        })
        .collect();

    // 119. AND THE SIX SAVES, with whatever is running on them. The
    // ability rows are about to roll these, so the number has to come
    // from the engine rather than be assembled on the screen.
    sheet.save_lines = crate::grants::ABILITIES
        .iter()
        .map(|code| (code.to_string(), sheet.save_line(code, &live)))
        .collect();

    sheet.karma = sheet.derive_karma(&class_catalogue);
    Ok(sheet)
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    /// Rodnar at level 5: PB +3. WIS 16 (+3), STR 8 (-1), DEX 14 (+2).
    /// Proficient in Insight, expertise in Perception, half in Stealth,
    /// untrained in Athletics. WIS saves proficient, STR saves not.
    fn rodnar() -> Sheet {
        let mut abilities = HashMap::new();
        abilities.insert("str".into(), Ability::plain(8, false));
        abilities.insert("dex".into(), Ability::plain(14, false));
        abilities.insert("con".into(), Ability::plain(15, false));
        abilities.insert("int".into(), Ability::plain(10, false));
        abilities.insert("wis".into(), Ability::plain(16, true));
        abilities.insert("cha".into(), Ability::plain(12, false));

        let skills = vec![
            SkillDef { key: "ins".into(), name: "Insight".into(),    ability: "wis".into() },
            SkillDef { key: "prc".into(), name: "Perception".into(), ability: "wis".into() },
            SkillDef { key: "ste".into(), name: "Stealth".into(),    ability: "dex".into() },
            SkillDef { key: "ath".into(), name: "Athletics".into(),  ability: "str".into() },
        ];

        let mut profs = HashMap::new();
        profs.insert("ins".into(), 1.0);
        profs.insert("prc".into(), 2.0);
        profs.insert("ste".into(), 0.5);

        Sheet {
            classes: Vec::new(),
            spells: Vec::new(),
            location_id: None,
            character_id: Some("c1".into()),
            game_id: "g1".into(),
            prof_bonus: None,
            name: "Rodnar Shieldcrest".into(),
            level: 5,
            // A PLAYER CHARACTER, so the gate is his own level.
            from_statblock: false,
            gate_level: 5,
            // A barbarian. Nothing in this fixture casts anything.
            caster: None,
            abilities,
            skills,
            profs,
            skill_mods: HashMap::new(),
            // Resolution is arithmetic and string matching; the prose
            // rides along on the sheet but has no part in it. The
            // lines themselves are tested in narrative.rs.
            narrative_pack: narrative::BASE_PACK.into(),
            narratives: HashMap::new(),
            // Rodnar's, from the 008 backfill. Resolution does not read
            // them yet - the attack key is the next job - but the sheet
            // carries them and the fixture should not lie about it. The
            // rules that DO read them are tested in equipment.rs.
            species: None,
            body: Body::default(),
            carry_size: None,
            tool_profs: Vec::new(),
            karma: None,
            weapon_profs: vec!["sim".into()],
            armor_profs: vec!["lgt".into(), "med".into(), "shl".into()],
            loadout: Vec::new(),
            techniques: Vec::new(),
            vitals: Vitals {
                hp_max: Some(74),
                hp_temp: 0,
                hp_temp_max: 0,
                ac_mode: "default".into(),
                // The export's flat value, carried so the fixture stays
                // faithful to the source. It is NOT his AC, and nothing
                // reads it while the mode is 'default'.
                ac_override: Some(14),
                death_successes: 0,
                death_failures: 0,
                exhaustion: 0,
                inspiration: false,
                size: Some("med".into()),
            },
            // Empty loadout, so 10 + DEX(+2). The armoured answer, and
            // the fact that it is 15 rather than the export's 14, are
            // tested where the rule lives in equipment.rs.
            armor_class: 12,
            // 122. Nobody has said what Rodnar is, which is the
            // honest state of every character until a DM says so.
            creature_type: None,
            // 116. Rodnar is human and wearing nothing enchanted, so
            // everything hurts him exactly as much as it says.
            resistances: Vec::new(),
            // 119. Derived off the finished sheet in load_sheet, the
            // same as skill_mods beside it. The save ARITHMETIC is
            // tested through `save_line` and `resolve_request`, which
            // ask the fixture rather than this map.
            save_lines: HashMap::new(),
        }
    }

    #[test]
    fn proficiency_bonus_by_level() {
        let mut s = rodnar();
        for (level, pb) in [(1, 2), (4, 2), (5, 3), (8, 3), (9, 4), (13, 5), (17, 6), (20, 6)] {
            s.level = level;
            assert_eq!(s.proficiency_bonus(), pb, "level {}", level);
        }
    }

    #[test]
    fn ability_modifiers_floor_toward_negative() {
        let s = rodnar();
        assert_eq!(s.ability_mod("wis"), 3);   // 16
        assert_eq!(s.ability_mod("dex"), 2);   // 14
        assert_eq!(s.ability_mod("con"), 2);   // 15 -> floor(2.5)
        assert_eq!(s.ability_mod("int"), 0);   // 10
        assert_eq!(s.ability_mod("str"), -1);  // 8  -> floor(-1.0)
    }

    #[test]
    fn odd_low_scores_round_the_right_way() {
        // The case plain integer division gets wrong: 7 must be -2, not
        // -1. Rust truncates toward zero; Math.floor does not.
        let mut s = rodnar();
        s.abilities.insert("str".into(), Ability::plain(7, false));
        assert_eq!(s.ability_mod("str"), -2);
        s.abilities.insert("str".into(), Ability::plain(3, false));
        assert_eq!(s.ability_mod("str"), -4);
        s.abilities.insert("str".into(), Ability::plain(1, false));
        assert_eq!(s.ability_mod("str"), -5);
    }

    #[test]
    fn proficient_skill_adds_full_bonus() {
        let r = resolve_request(&rodnar(), "insight", "normal");
        assert_eq!(r.label, "Insight (WIS)");
        assert_eq!(r.modifier, 6);            // WIS +3, PB +3
        assert_eq!(r.formula, "1d20+6");
    }

    #[test]
    fn skill_resolves_by_key_as_well_as_name() {
        let by_key = resolve_request(&rodnar(), "ins", "normal");
        let by_name = resolve_request(&rodnar(), "Insight", "normal");
        assert_eq!(by_key, by_name);
    }

    #[test]
    fn expertise_doubles_the_bonus() {
        let r = resolve_request(&rodnar(), "perception", "normal");
        assert_eq!(r.modifier, 9);            // WIS +3, PB +3 doubled
        assert_eq!(r.formula, "1d20+9");
    }

    #[test]
    fn half_proficiency_floors() {
        // 0.5 * 3 = 1.5 -> 1, not 2 and not 1.5.
        let r = resolve_request(&rodnar(), "stealth", "normal");
        assert_eq!(r.modifier, 3);            // DEX +2, half PB +1
        assert_eq!(r.formula, "1d20+3");
    }

    #[test]
    fn untrained_skill_is_the_bare_ability() {
        let r = resolve_request(&rodnar(), "athletics", "normal");
        assert_eq!(r.modifier, -1);           // STR -1, no proficiency
        assert_eq!(r.formula, "1d20-1");
    }

    #[test]
    fn advantage_and_disadvantage_change_only_the_dice() {
        let adv = resolve_request(&rodnar(), "insight", "adv");
        let dis = resolve_request(&rodnar(), "insight", "dis");
        assert_eq!(adv.formula, "2d20kh1+6");
        assert_eq!(dis.formula, "2d20kl1+6");
        assert_eq!(adv.modifier, dis.modifier);
        assert_eq!(adv.label, dis.label);
    }

    #[test]
    fn proficient_save_adds_the_bonus() {
        let r = resolve_request(&rodnar(), "wis save", "normal");
        assert_eq!(r.label, "WIS Save");
        assert_eq!(r.modifier, 6);            // WIS +3, proficient
    }

    /* ---------------- a save against a spell ---------------- */

    /// A Ny'ook: +2 on a save against a spell, and nothing on any
    /// other save. 098, after Dave chose the +2 over the advantage
    /// the source also claimed.
    fn nyook() -> Sheet {
        let mut s = rodnar();
        let mut sp = crate::species::from_row(&serde_json::json!({
            "key": "nyook", "name": "Ny'ook",
            "ability_bonuses": {}, "size": "sm",
            "spell_save_bonus": 2, "playable": true
        }));
        sp.spell_save_bonus = Some(2);
        s.species = Some(sp);
        s
    }

    /* ---------------- what the magic adds ---------------- */

    /// One worn thing that grants something.
    fn wearing(grants: Vec<crate::grants::Grant>) -> Vec<equipment::Owned> {
        vec![equipment::Owned {
            id: "o1".into(),
            name: Some("Gauntlets of Ogre Power".into()),
            item: equipment::blank("Gauntlets of Ogre Power", "equipment"),
            quantity: 1,
            slot: Some("hands".into()),
            equipped: true,
            attuned: true,
            grants,
            proficient_override: None,
            proficient: true,
            modes: vec![],
            uses_spent: 0,
            uses_max: None,
        }]
    }

    fn grant(target: &str, mode: crate::grants::Mode, value: i64) -> crate::grants::Grant {
        crate::grants::Grant {
            target: target.into(),
            mode,
            value,
            dice: None,
            needs_attunement: false,
            source: "Gauntlets of Ogre Power".into(),
        }
    }

    #[test]
    fn an_item_can_set_a_score_it_finds_too_low() {
        let mut a = HashMap::new();
        a.insert("str".to_string(), Ability::plain(8, false));
        apply_grants(&mut a, &wearing(vec![grant("str", crate::grants::Mode::Set, 19)]));
        assert_eq!(a["str"].score, 19);
        assert_eq!(a["str"].base, 8, "what they rolled is untouched");
        assert_eq!(a["str"].sources.len(), 1);
        assert_eq!(a["str"].sources[0].name, "Gauntlets of Ogre Power");
    }

    #[test]
    fn an_item_that_sets_lower_than_you_are_does_nothing() {
        let mut a = HashMap::new();
        a.insert("str".to_string(), Ability::plain(20, false));
        apply_grants(&mut a, &wearing(vec![grant("str", crate::grants::Mode::Set, 19)]));
        assert_eq!(a["str"].score, 20);
        assert!(a["str"].sources.is_empty(), "nothing happened, nothing claimed");
    }

    /// THE WHOLE POINT OF THE ORDER. 099 stops a Ny'ook writing a
    /// Strength above 13; Dave's rule in the same breath was that an
    /// item may carry them past it. The cap binds the natural score
    /// and the grant lands on top of the capped result.
    #[test]
    fn magic_carries_a_capped_people_past_their_ceiling() {
        let mut a = HashMap::new();
        a.insert("str".to_string(), Ability::plain(13, false));
        apply_grants(&mut a, &wearing(vec![grant("str", crate::grants::Mode::Set, 19)]));
        assert_eq!(a["str"].score, 19, "a Ny'ook in gauntlets");
    }

    #[test]
    fn nothing_worn_changes_nothing() {
        let mut a = HashMap::new();
        a.insert("str".to_string(), Ability::plain(12, false));
        apply_grants(&mut a, &[]);
        assert_eq!(a["str"].score, 12);
        assert!(a["str"].sources.is_empty());
    }

    #[test]
    fn a_grant_on_another_ability_leaves_this_one_alone() {
        let mut a = HashMap::new();
        a.insert("str".to_string(), Ability::plain(12, false));
        a.insert("int".to_string(), Ability::plain(10, false));
        apply_grants(&mut a, &wearing(vec![grant("int", crate::grants::Mode::Set, 19)]));
        assert_eq!(a["str"].score, 12);
        assert_eq!(a["int"].score, 19);
    }

    #[test]
    fn a_people_can_be_hard_to_enchant() {
        let r = resolve_request(&nyook(), "wis save vs spell", "normal");
        assert_eq!(r.label, "WIS Save vs Spell");
        assert_eq!(r.modifier, 8, "WIS +3, proficient +3, and the people's +2");
    }

    #[test]
    fn the_bonus_waits_to_be_told_what_the_save_is_against() {
        // The same creature, the same save, no circumstance given.
        let r = resolve_request(&nyook(), "wis save", "normal");
        assert_eq!(r.label, "WIS Save");
        assert_eq!(r.modifier, 6);
    }

    #[test]
    fn everybody_elses_save_against_a_spell_is_an_ordinary_save() {
        let r = resolve_request(&rodnar(), "wis save vs spell", "normal");
        assert_eq!(r.modifier, 6, "no people, no bonus");
        // Still labelled, because what the save was against is worth
        // reading back off the card whoever rolled it.
        assert_eq!(r.label, "WIS Save vs Spell");
    }

    /* ---------- 119. the save line the sheet shows ---------- */

    fn saver(grants: Vec<crate::grants::Grant>) -> Sheet {
        Sheet { loadout: wearing(grants), ..rodnar() }
    }

    fn named(target: &str, value: i64, who: &str) -> crate::grants::Grant {
        crate::grants::Grant {
            target: target.into(),
            mode: crate::grants::Mode::Add,
            value,
            dice: None,
            needs_attunement: false,
            source: who.into(),
        }
    }

    #[test]
    fn a_bare_save_is_the_ability_and_the_proficiency() {
        // Rodnar: WIS 16 (+3), proficient, PB +3.
        let line = rodnar().save_line("wis", &[]);
        assert_eq!(line.natural, 6);
        assert_eq!(line.total, 6);
        assert!(line.proficient);
        assert!(line.mods.is_empty(), "nothing moved it");
    }

    #[test]
    fn an_unproficient_save_leaves_the_bonus_out_and_says_so() {
        // STR 8 is -1, and Rodnar is not proficient in STR saves.
        let line = rodnar().save_line("str", &[]);
        assert_eq!(line.natural, -1);
        assert!(!line.proficient);
    }

    /// THE WHOLE POINT OF THE COLUMN. A +1 that appears in a total
    /// with nothing to account for it reads as the arithmetic being
    /// wrong - which is why the attack preview has spelled out STR and
    /// proficiency separately since 100.
    #[test]
    fn every_modifier_arrives_with_its_name_on_it() {
        let line = saver(vec![named("save", 1, "Cloak of Protection")])
            .save_line("wis", &[]);
        assert_eq!(line.total, 7);
        assert_eq!(line.mods.len(), 1);
        assert_eq!(line.mods[0].name, "Cloak of Protection");
        assert_eq!(line.mods[0].value, Some(1));
    }

    #[test]
    fn two_items_doing_different_jobs_both_land() {
        // 100's two targets: a cloak is every save, a ring is one.
        let line = saver(vec![
            named("save", 1, "Cloak of Protection"),
            named("save.wis", 2, "a steadying ring"),
        ])
        .save_line("wis", &[]);
        assert_eq!(line.total, 9);
        assert_eq!(line.mods.len(), 2);
    }

    #[test]
    fn a_ring_for_another_ability_is_not_this_saves_business() {
        let line = saver(vec![named("save.dex", 2, "a steadying ring")])
            .save_line("wis", &[]);
        assert_eq!(line.total, 6);
        assert!(line.mods.is_empty());
    }

    /// 114. A DIE IS NOT A NUMBER and must never be averaged into one.
    /// `effect_grants` appends it to the formula at roll time so the
    /// roll shows what it rolled.
    #[test]
    fn a_running_bless_is_shown_as_a_die_and_not_added_to_the_total() {
        let bless = crate::grants::Grant {
            target: "save".into(),
            mode: crate::grants::Mode::Add,
            value: 0,
            dice: Some("1d4".into()),
            needs_attunement: false,
            source: "Bless".into(),
        };
        let line = rodnar().save_line("wis", &[bless]);
        assert_eq!(line.total, 6, "the die is not worth a number");
        assert_eq!(line.mods.len(), 1);
        assert_eq!(line.mods[0].dice.as_deref(), Some("1d4"));
        assert_eq!(line.mods[0].value, None);
    }

    /// THE SHEET COUNTS IT AND THE ROLL DOES NOT, which is the one
    /// place these two callers must differ: `effect_grants` puts a
    /// running Bless into the formula, so `resolve_request` passing it
    /// here as well would roll the d4 twice.
    #[test]
    fn the_roll_path_is_told_nothing_about_live_effects() {
        let r = resolve_request(&rodnar(), "wis save", "normal");
        assert_eq!(r.modifier, rodnar().save_line("wis", &[]).total);
    }

    #[test]
    fn a_conditional_bonus_is_listed_but_not_in_the_total() {
        // A Ny'ook's +2 is real and is not part of a bare save.
        let line = nyook().save_line("wis", &[]);
        assert_eq!(line.total, 6, "a bare save is unchanged");
        assert_eq!(line.conditional(), 2);
        let only = line.mods.iter().find(|m| m.only_vs_spell).unwrap();
        assert_eq!(only.name, "Ny'ook");
    }

    #[test]
    fn a_people_with_no_such_bonus_lists_nothing_conditional() {
        let line = rodnar().save_line("wis", &[]);
        assert_eq!(line.conditional(), 0);
        assert!(line.mods.iter().all(|m| !m.only_vs_spell));
    }

    /// ONE ANSWER, WHICHEVER WAY YOU ASK. The sheet's column and the
    /// roll box must never disagree - that is the fault `skill_mods`
    /// exists to have fixed once already, where four places derived a
    /// skill modifier and three of them asked the engine.
    #[test]
    fn the_line_and_the_roll_agree_for_every_ability_both_ways() {
        let sheets = [
            ("rodnar", rodnar()),
            ("nyook", nyook()),
            ("laden", saver(vec![named("save", 1, "Cloak of Protection")])),
        ];
        for (who, sheet) in sheets {
            for code in crate::grants::ABILITIES {
                let line = sheet.save_line(code, &[]);

                let plain = resolve_request(&sheet, &format!("{} save", code), "normal");
                assert_eq!(plain.modifier, line.total, "{} {} save", who, code);

                let magic =
                    resolve_request(&sheet, &format!("{} save vs spell", code), "normal");
                assert_eq!(
                    magic.modifier,
                    line.total + line.conditional(),
                    "{} {} save vs spell",
                    who, code
                );
            }
        }
    }

    #[test]
    fn four_ways_to_say_it() {
        for req in [
            "wis save vs spell",
            "wis save vs spells",
            "wis save against spell",
            "wis save against spells",
        ] {
            assert_eq!(resolve_request(&nyook(), req, "normal").modifier, 8, "{}", req);
        }
    }

    /// THE KEY MUST NOT FORK. It is the vocabulary narrative_lines and
    /// skill_prompts are written in, and a `wis_save_spell` nobody
    /// seeded would cost a character their prose on the one roll that
    /// mattered.
    #[test]
    fn the_circumstance_does_not_change_the_roll_key() {
        assert_eq!(resolve_request(&nyook(), "wis save", "normal").key, "wis_save");
        assert_eq!(
            resolve_request(&nyook(), "wis save vs spell", "normal").key,
            "wis_save"
        );
    }

    #[test]
    fn unproficient_save_does_not() {
        let r = resolve_request(&rodnar(), "str save", "normal");
        assert_eq!(r.label, "STR Save");
        assert_eq!(r.modifier, -1);
    }

    #[test]
    fn long_ability_names_work_too() {
        assert_eq!(
            resolve_request(&rodnar(), "wisdom save", "normal"),
            resolve_request(&rodnar(), "wis save", "normal")
        );
        assert_eq!(
            resolve_request(&rodnar(), "strength check", "normal").label,
            "STR Check"
        );
    }

    #[test]
    fn ability_check_never_adds_proficiency() {
        let r = resolve_request(&rodnar(), "wis", "normal");
        assert_eq!(r.label, "WIS Check");
        assert_eq!(r.modifier, 3);            // not 6 — a check is not a save
    }

    #[test]
    fn a_skill_is_not_shadowed_by_an_ability_check() {
        // "ins" is a skill key; it must not be mistaken for anything
        // else. Ordering in resolve_request is what guarantees this.
        let r = resolve_request(&rodnar(), "ins", "normal");
        assert_eq!(r.label, "Insight (WIS)");
    }

    #[test]
    fn unknown_requests_pass_through_as_formulas() {
        let r = resolve_request(&rodnar(), "2d6+3", "normal");
        assert_eq!(r.formula, "2d6+3");
        assert_eq!(r.label, "2d6+3");
        assert_eq!(r.modifier, 0);
    }

    /* ---------------------- 154. SPELL ATTACKS ---------------------- */

    /// A cleric with the three Attack spells and one that is not.
    fn luci() -> Sheet {
        let mut s = rodnar();
        s.name = "Luci".into();
        s.caster = Some(crate::casting::Caster {
            class_key: "cleric".into(),
            level: 5,
            ability: "wis".into(),
            source: crate::casting::Source::WholeList,
        });
        s.spells = vec![
            crate::spellcast::Known {
                key: "sp_spiritualweapon".into(),
                name: "Spiritual Weapon".into(),
                roll_name: "Spiritual Weapon".into(),
                level: 2,
                cast_type: "Attack".into(),
                casting_time: Some("1 bonus action".into()),
                save_ability: None,
                dice: Some("1d8".into()),
                range: Some("60 feet".into()),
            },
            crate::spellcast::Known {
                key: "sp_inflictwounds".into(),
                name: "Inflict Wounds".into(),
                roll_name: "Inflict Wounds".into(),
                level: 1,
                cast_type: "Attack".into(),
                casting_time: Some("1 action".into()),
                save_ability: None,
                dice: Some("3d10".into()),
                range: Some("Touch".into()),
            },
            crate::spellcast::Known {
                key: "sp_bless".into(),
                name: "Bless".into(),
                roll_name: "Bless".into(),
                level: 1,
                cast_type: "Utility".into(),
                casting_time: Some("1 action".into()),
                save_ability: None,
                dice: None,
                range: Some("30 feet".into()),
            },
        ];
        s
    }

    /// The bug: Luci cast it, the slot went, the log promised "+6 to
    /// hit, 1d8", and the roll box was handed the literal text
    /// "spiritual weapon" because nothing knew what it was.
    #[test]
    fn an_attack_spell_resolves_as_an_attack() {
        let r = resolve_request(&luci(), "Spiritual Weapon", "normal");
        // WIS 16 is +3 and a level-5 character's bonus is +3.
        assert_eq!(r.modifier, 6);
        assert_eq!(r.formula, "1d20+6");
        assert_eq!(r.label, "Spiritual Weapon \u{2014} +6 to hit, 1d8");
        let a = r.attack.expect("a spell attack carries an Attack");
        assert_eq!(a.damage, "1d8");
        assert_eq!(a.to_hit, 6);
        assert!(a.proficient, "nobody casts their own spell unproficiently");
    }

    /// The key is `attack` and not `spell_attack`, for the same reason
    /// the save key does not fork: prose is seeded against it.
    #[test]
    fn a_spell_attack_takes_the_attack_prose() {
        assert_eq!(resolve_request(&luci(), "Spiritual Weapon", "normal").key, "attack");
    }

    /// 143 made "from nonmagical attacks" expressible. A spectral mace
    /// is magical, so a werewolf does not halve it.
    #[test]
    fn a_spell_attack_is_magical() {
        let a = resolve_request(&luci(), "Inflict Wounds", "normal").attack.unwrap();
        assert!(a.magical);
        assert_eq!(a.magic, 0, "magical is not the same as carrying a plus");
    }

    /// A Utility spell rolls no d20 and must not pretend to. Bless falls
    /// through to the raw formula, where the dice engine refuses it.
    #[test]
    fn a_spell_that_makes_no_attack_roll_does_not_resolve_as_one() {
        let r = resolve_request(&luci(), "Bless", "normal");
        assert!(r.attack.is_none());
        assert_eq!(r.key, "custom");
    }

    /// The branch is gated on being a caster at all, so a barbarian who
    /// types a spell name gets the old behaviour rather than a panic.
    #[test]
    fn somebody_who_does_not_cast_gets_no_spell_branch() {
        let r = resolve_request(&rodnar(), "Spiritual Weapon", "normal");
        assert!(r.attack.is_none());
        assert_eq!(r.key, "custom");
    }

    /// Matched the way every other branch matches.
    #[test]
    fn a_spell_is_found_regardless_of_case_or_padding() {
        for req in ["spiritual weapon", "  SPIRITUAL WEAPON  ", "Spiritual Weapon"] {
            assert_eq!(
                resolve_request(&luci(), req, "normal").modifier,
                6,
                "{}",
                req
            );
        }
    }

    /// Advantage still reaches a spell attack - it is a d20 like any
    /// other, which is the whole argument for putting it here.
    #[test]
    fn a_spell_attack_takes_advantage() {
        let adv = resolve_request(&luci(), "Spiritual Weapon", "adv");
        assert_eq!(adv.formula, "2d20kh1+6");
    }

    /// A weapon wins a name collision, because the attack branch runs
    /// first and a loadout is the more specific thing.
    #[test]
    fn a_weapon_of_the_same_name_still_wins() {
        let mut s = luci();
        s.spells[0].roll_name = "insight".into();
        s.spells[0].name = "insight".into();
        // The skill branch is BELOW the spell branch, so this proves the
        // spell is reached first and the ordering is deliberate.
        assert_eq!(resolve_request(&s, "insight", "normal").key, "attack");
    }

    #[test]
    fn case_and_whitespace_do_not_matter() {
        let r = resolve_request(&rodnar(), "  INSIGHT  ", "normal");
        assert_eq!(r.label, "Insight (WIS)");
        assert_eq!(r.formula, "1d20+6");
    }

    #[test]
    fn the_key_is_engine_vocabulary_not_what_was_typed() {
        // narrative_lines.key and skill_prompts.key are written in this
        // vocabulary. Every spelling of a request must land on one key
        // or a pack would have to carry a line per synonym.
        for req in ["insight", "Insight", "ins", "  INSIGHT  "] {
            assert_eq!(resolve_request(&rodnar(), req, "normal").key, "ins", "{}", req);
        }
        assert_eq!(resolve_request(&rodnar(), "wis save", "normal").key, "wis_save");
        assert_eq!(resolve_request(&rodnar(), "wisdom save", "normal").key, "wis_save");
        assert_eq!(resolve_request(&rodnar(), "str check", "normal").key, "str_check");
        assert_eq!(resolve_request(&rodnar(), "wis", "normal").key, "wis_check");
        assert_eq!(resolve_request(&rodnar(), "2d6+3", "normal").key, "custom");
    }

    #[test]
    fn a_level_one_character_has_pb_two() {
        let mut s = rodnar();
        s.level = 1;
        let r = resolve_request(&s, "insight", "normal");
        assert_eq!(r.modifier, 5);            // WIS +3, PB +2
    }

    /* ------------------ the unarmoured floor ------------------ */

    /// Garn's people: +2 STR, +1 CON, and Unyielding Defense at 12 +
    /// CON. The same row species.rs tests against.
    fn untgaroth() -> crate::species::Species {
        crate::species::from_row(&serde_json::json!({
            "key": "untgaroth", "name": "Unt'garoth",
            "ability_bonuses": { "str": 2, "con": 1 },
            "ability_maxima": { "str": 21 },
            "size": "lg", "carry_size_steps": 1, "skill_profs": ["ath"],
            "unarmored_ac_base": 12, "unarmored_ac_ability": "con",
            "playable": true
        }))
    }

    /// Garn as stored: CON 13 on the row, 14 once his people are in it.
    fn garn() -> HashMap<String, Ability> {
        let mut a = HashMap::new();
        a.insert("str".to_string(), Ability::plain(18, false));
        a.insert("dex".to_string(), Ability::plain(11, false));
        a.insert("con".to_string(), Ability::plain(13, false));
        a
    }

    #[test]
    fn no_species_leaves_the_ordinary_floor_alone() {
        assert_eq!(unarmored_rule(None, &garn()), None);
    }

    #[test]
    fn a_people_without_the_rule_claims_nothing() {
        let mut sp = untgaroth();
        sp.unarmored_ac_base = None;
        assert_eq!(unarmored_rule(Some(&sp), &garn()), None);
    }

    /// THE BUG THIS PAIR IS FOR. The roster read the STORED score and
    /// no species at all, so Garn's sheet said AC 14 while the target
    /// list the goblins roll against said 10 - and the target list is
    /// the one that decides whether a swing connected.
    #[test]
    fn a_fighter_who_took_one_level_of_bard_still_has_karma() {
        // 076, and the bug this test exists for. Falon is Fighter 4 /
        // Bard 1 - the fighter LEADS, because four beats one - and the
        // first version of derive_karma found the leading class and
        // then asked whether it had a formula. It does not, so it
        // returned None and a bard with a sword had no Karma.
        let mut sheet = rodnar();
        sheet.classes = vec![
            crate::multiclass::Taken { key: "fighter".into(), level: 4, hit_die: 10 },
            crate::multiclass::Taken { key: "bard".into(), level: 1, hit_die: 8 },
        ];
        let catalogue = vec![
            crate::class::from_row(&serde_json::json!({
                "key": "fighter", "name": "Fighter", "hit_die": 10, "karma_skills": []
            })),
            crate::class::from_row(&serde_json::json!({
                "key": "bard", "name": "Bard", "hit_die": 8, "karma_skills": ["ins", "prf"]
            })),
        ];

        let k = sheet.derive_karma(&catalogue).expect("the bard level answers");
        assert_eq!(k.class_key, "bard");
        assert_eq!(k.parts.len(), 2);
    }

    #[test]
    fn a_character_with_no_class_that_expresses_karma_has_none() {
        let mut sheet = rodnar();
        sheet.classes = vec![crate::multiclass::Taken {
            key: "fighter".into(), level: 4, hit_die: 10,
        }];
        let catalogue = vec![crate::class::from_row(&serde_json::json!({
            "key": "fighter", "name": "Fighter", "hit_die": 10, "karma_skills": []
        }))];
        assert!(sheet.derive_karma(&catalogue).is_none());
    }

    #[test]
    fn karma_sums_the_modifiers_the_class_names() {
        // Rodnar's fixture, read as a bard. Whatever his Insight and
        // Performance modifiers are, Karma is their sum - which pins
        // that this reads the SHEET's skill rule and not a second copy.
        let mut sheet = rodnar();
        sheet.classes = vec![crate::multiclass::Taken {
            key: "bard".into(), level: 3, hit_die: 8,
        }];
        let catalogue = vec![crate::class::from_row(&serde_json::json!({
            "key": "bard", "name": "Bard", "hit_die": 8, "karma_skills": ["ins", "prf"]
        }))];

        let k = sheet.derive_karma(&catalogue).unwrap();
        let expected = sheet.skill_modifier("ins") + sheet.skill_modifier("prf");
        assert_eq!(k.raw, expected);
        assert_eq!(k.rating, expected.clamp(0, crate::karma::MAX));
    }

    #[test]
    fn the_floor_reads_the_effective_score_not_the_stored_one() {
        let sp = untgaroth();
        let mut abilities = garn();

        // Stored CON 13 is +1. Nobody should ever see this number.
        assert_eq!(unarmored_rule(Some(&sp), &abilities), Some((12, 1)));

        // With their people applied, CON 14 is +2 - and that is what
        // both screens now compute from.
        apply_species(&mut abilities, &sp);
        assert_eq!(unarmored_rule(Some(&sp), &abilities), Some((12, 2)));
    }

    /// The four sites that ask this loader - the target list, the
    /// initiative roll, the level button and the hit-point
    /// rederivation - all lean on the same two defaults, so both are
    /// stated here rather than rediscovered at each one.
    #[test]
    fn the_loader_answers_for_everybody_including_the_unknown() {
        let sp = untgaroth();
        let mut abilities = garn();
        apply_species(&mut abilities, &sp);

        let mut scores = HashMap::new();
        scores.insert("garn".to_string(), abilities);
        let mut peoples = HashMap::new();
        peoples.insert("garn".to_string(), sp);
        let eff = Effective::of(scores, peoples);

        assert_eq!(eff.modifier("garn", "con"), 2, "13 stored, 14 with his people");
        assert_eq!(eff.modifier("garn", "str"), 5, "18 stored, 20 with his people");
        assert_eq!(eff.unarmored("garn"), Some((12, 2)));

        // A creature with no rows at all - every monster - reads zero
        // and no floor, which is what all four sites did before.
        assert_eq!(eff.modifier("a goblin", "dex"), 0);
        assert_eq!(eff.unarmored("a goblin"), None);
    }

    /// THE SCORE AND THE MODIFIER ARE DIFFERENT ANSWERS, and carrying
    /// capacity is the rule that wants the first: Strength times
    /// fifteen, where every other rule in the app wants the +5.
    #[test]
    fn the_loader_gives_the_score_as_well_as_the_modifier() {
        let sp = untgaroth();
        let mut abilities = garn();
        apply_species(&mut abilities, &sp);

        let mut scores = HashMap::new();
        scores.insert("garn".to_string(), abilities);
        let mut peoples = HashMap::new();
        peoples.insert("garn".to_string(), sp);
        let eff = Effective::of(scores, peoples);

        assert_eq!(eff.score("garn", "str"), 20, "18 on the row, 20 with his people");
        assert_eq!(eff.modifier("garn", "str"), 5);

        // The people itself, for carrying size - the reason this
        // accessor exists at all.
        assert_eq!(eff.people("garn").map(|s| s.carry_size()), Some("huge".to_string()));

        // A monster: ten and nobody, which is what the carry path read
        // before it asked the loader.
        assert_eq!(eff.score("a goblin", "str"), 10);
        assert!(eff.people("a goblin").is_none());
    }

    /// A character whose people states no floor keeps 5e's.
    #[test]
    fn a_people_without_a_floor_leaves_the_ordinary_one() {
        let mut sp = untgaroth();
        sp.unarmored_ac_base = None;
        let mut scores = HashMap::new();
        scores.insert("x".to_string(), garn());
        let mut peoples = HashMap::new();
        peoples.insert("x".to_string(), sp);
        assert_eq!(Effective::of(scores, peoples).unarmored("x"), None);
    }

    #[test]
    fn both_screens_arrive_at_the_same_armour_class() {
        let sp = untgaroth();
        let mut abilities = garn();
        apply_species(&mut abilities, &sp);

        let ac = crate::equipment::armor_class(
            ability_mod_of(&abilities, "dex"),
            &[],
            crate::equipment::AcMode::Default,
            None,
            unarmored_rule(Some(&sp), &abilities),
        );
        assert_eq!(ac, 14, "12 + CON 14's +2, unarmoured");

        // What the encounter path used to hand in: no species, DEX 11.
        let was = crate::equipment::armor_class(
            0,
            &[],
            crate::equipment::AcMode::Default,
            None,
            None,
        );
        assert_eq!(was, 10, "the four points a goblin was being given");
    }

    #[test]
    fn resolved_formulas_are_always_rollable() {
        // The contract between this module and dice.rs: anything
        // resolve_request emits, roll_formula must accept.
        let s = rodnar();
        for req in ["insight", "perception", "stealth", "athletics", "wis save", "str", "2d6+3"] {
            for mode in ["normal", "adv", "dis"] {
                let r = resolve_request(&s, req, mode);
                crate::dice::roll_formula(&r.formula).unwrap_or_else(|e| {
                    panic!("{} / {} produced unrollable {}: {}", req, mode, r.formula, e)
                });
            }
        }
    }
}
