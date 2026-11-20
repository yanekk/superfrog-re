//! Game ▸ Start Level…: the window that starts a level with chosen values, and the recorder on or
//! off (bug-recorder DESIGN §2.1, D1). The window is the toolkit's (`amiga_app::start_level_window`,
//! moved there by forest-dumb-level-1 T12), built from the fields [`spec`] names; this file says
//! what Superfrog's fields are and what their values mean: AppKit dropdowns, a field, checkboxes
//! and two buttons.
//!
//! ```text
//!   Level            [ W1-1     ▾]
//!   Lives            [ 3        ▾]
//!   Points           [ 0         ]  0–999 999      ← shown only while the field is not a number
//!   Energy (hearts)  [ 4        ▾]                    in range, and Start is disabled then
//!   Difficulty       [ Normal   ▾]
//!                    [ ] Spud
//!                    [ ] Wings
//!                    [x] Enable recorder?
//!                                  [ Cancel ] [ Start ]
//! ```
//!
//! The window is modal to the game: the app runs it in AppKit's modal loop, so no frame is stepped
//! while it is open (the game is paused) and `NativeUi::start_level` returns the answer as the
//! other panels do. Return is Start, Esc Cancel. It always opens on a new game's values.
//!
//! The model ([`StartLevelForm`]) holds no AppKit and is what the tests drive: the toolkit's
//! [`Form`] over [`spec`], read back as a [`LevelSetup`]. A script drives it with `form …` lines
//! (`script.rs`), answering the next Game ▸ Start Level… as the clicks would.

use amiga_app::start_level_window::{FieldAction, FieldKind, Form, LevelField, StartLevelSpec, Value, Values, whole_number};
use sf_core::LevelId;
use sf_core::recording::{Difficulty, LevelSetup, Lives, MAX_POINTS, levels};

/// What the window shows beside Points while it does not hold a whole number in range.
pub const POINTS_RANGE: &str = "0–999 999";

/// The Lives dropdown, top to bottom.
pub const LIVES: [Lives; 3] = [Lives::Three, Lives::Five, Lives::Seven];
/// The Difficulty dropdown: no Hard (D12).
pub const DIFFICULTIES: [Difficulty; 2] = [Difficulty::Easy, Difficulty::Normal];

/// What the person did in the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormAction {
    Level(LevelId),
    Lives(Lives),
    /// What the Points field holds now, as typed.
    Points(String),
    Energy(u8),
    Difficulty(Difficulty),
    Spud(bool),
    Wings(bool),
    Record(bool),
    Start,
    Cancel,
}

/// The fields, top to bottom, by their index in [`spec`]. Points sits between Lives and Energy, as
/// in the prototype.
const LEVEL: usize = 0;
const LIVES_FIELD: usize = 1;
const POINTS: usize = 2;
const ENERGY: usize = 3;
const DIFFICULTY: usize = 4;
const SPUD: usize = 5;
const WINGS: usize = 6;
const RECORD: usize = 7;

/// The window's fields, on a new game's values (DESIGN §2.1) with the recorder on.
pub fn spec() -> StartLevelSpec {
    let s = LevelSetup::default();
    let dropdown = |id: &str, label: &str, titles: Vec<String>, at: usize| LevelField {
        id: id.into(),
        label: label.into(),
        kind: FieldKind::Dropdown(titles),
        default: Value::Choice(at),
    };
    let checkbox = |id: &str, label: &str, on: bool| LevelField {
        id: id.into(),
        label: label.into(),
        kind: FieldKind::Checkbox,
        default: Value::On(on),
    };
    StartLevelSpec {
        fields: vec![
            dropdown(
                "level",
                "Level",
                levels().iter().map(ToString::to_string).collect(),
                levels().iter().position(|&l| l == s.level).unwrap_or(0),
            ),
            dropdown(
                "lives",
                "Lives",
                LIVES.iter().map(|l| l.count().to_string()).collect(),
                LIVES.iter().position(|&l| l == s.lives).unwrap_or(0),
            ),
            LevelField {
                id: "points".into(),
                label: "Points".into(),
                kind: FieldKind::Number { max: MAX_POINTS, range: POINTS_RANGE.into() },
                default: Value::Text(s.points.to_string()),
            },
            dropdown("energy", "Energy (hearts)", (1..=4).map(|e: u8| e.to_string()).collect(), s.energy as usize - 1),
            dropdown(
                "difficulty",
                "Difficulty",
                DIFFICULTIES.iter().map(|&d| difficulty_title(d).into()).collect(),
                DIFFICULTIES.iter().position(|&d| d == s.difficulty).unwrap_or(0),
            ),
            checkbox("spud", "Spud", s.spud),
            checkbox("wings", "Wings", s.wings),
            checkbox("record", "Enable recorder?", true),
        ],
    }
}

/// The values the window gives back, as a level's start and whether to record. `points` stands in
/// for a Points field out of range.
fn setup_of(v: &Values, points: u32) -> (LevelSetup, bool) {
    let d = LevelSetup::default();
    let setup = LevelSetup {
        level: v.choice(LEVEL).and_then(|i| levels().get(i).copied()).unwrap_or(d.level),
        lives: v.choice(LIVES_FIELD).and_then(|i| LIVES.get(i).copied()).unwrap_or(d.lives),
        points: v.text(POINTS).and_then(|t| whole_number(t, MAX_POINTS)).unwrap_or(points),
        energy: v.choice(ENERGY).map_or(d.energy, |i| i as u8 + 1),
        difficulty: v.choice(DIFFICULTY).and_then(|i| DIFFICULTIES.get(i).copied()).unwrap_or(d.difficulty),
        spud: v.on(SPUD).unwrap_or(d.spud),
        wings: v.on(WINGS).unwrap_or(d.wings),
    };
    (setup, v.on(RECORD).unwrap_or(true))
}

/// The window's values: the toolkit's form over [`spec`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartLevelForm {
    form: Form,
    /// The points as last valid, which the setup keeps while the field is refused.
    points: u32,
}

impl Default for StartLevelForm {
    fn default() -> StartLevelForm {
        StartLevelForm::new()
    }
}

impl StartLevelForm {
    /// A new game's values (DESIGN §2.1), the recorder on.
    pub fn new() -> StartLevelForm {
        StartLevelForm { form: Form::new(spec()), points: LevelSetup::default().points }
    }

    /// Every value but the points as chosen; the points as last valid.
    #[cfg(test)]
    pub fn setup(&self) -> LevelSetup {
        setup_of(&self.form.values, self.points).0
    }

    /// Enable recorder?
    #[cfg(test)]
    pub fn record(&self) -> bool {
        setup_of(&self.form.values, self.points).1
    }

    /// The Points field's text.
    #[cfg(test)]
    pub fn points_text(&self) -> &str {
        self.form.values.text(POINTS).unwrap_or_default()
    }

    /// The Points field as a score: a whole number 0–999 999, `None` otherwise.
    pub fn points(&self) -> Option<u32> {
        self.form.number(POINTS)
    }

    /// Start is enabled only with points in range.
    #[cfg(test)]
    pub fn can_start(&self) -> bool {
        self.form.can_start()
    }

    /// What shows beside the Points field: the range while it is refused, else nothing.
    #[cfg(test)]
    pub fn points_hint(&self) -> &str {
        self.form.hint(POINTS)
    }

    /// A click, a choice or a key in the window: `None` while it stays open, `Some(None)` for
    /// Cancel, `Some(Some(..))` for Start with the values and whether to record. Start with points
    /// out of range does nothing, as its disabled button would; a level or an energy the dropdowns
    /// do not offer changes nothing.
    pub fn apply(&mut self, a: FormAction) -> Option<Option<(LevelSetup, bool)>> {
        let done = self.form.apply(a.field()?);
        if let Some(p) = self.points() {
            self.points = p;
        }
        done.map(|v| v.map(|v| setup_of(&v, self.points)))
    }
}

/// Run the window until Start or Cancel: the values and whether to record, `None` for Cancel.
pub fn run_modal(parent: Option<&winit::window::Window>) -> Option<(LevelSetup, bool)> {
    let v = amiga_app::start_level_window::run_modal("superfrog", &spec(), parent)?;
    Some(setup_of(&v, LevelSetup::default().points))
}

impl FormAction {
    /// The window's action it is; `None` for a level or an energy the dropdowns do not offer.
    pub fn field(&self) -> Option<FieldAction> {
        let set = |i, v| Some(FieldAction::Set(i, v));
        match self {
            FormAction::Level(id) => set(LEVEL, Value::Choice(levels().iter().position(|l| l == id)?)),
            FormAction::Lives(l) => set(LIVES_FIELD, Value::Choice(LIVES.iter().position(|x| x == l)?)),
            FormAction::Points(t) => set(POINTS, Value::Text(t.clone())),
            FormAction::Energy(e) if (1..=4).contains(e) => set(ENERGY, Value::Choice(*e as usize - 1)),
            FormAction::Energy(_) => None,
            FormAction::Difficulty(d) => set(DIFFICULTY, Value::Choice(DIFFICULTIES.iter().position(|x| x == d)?)),
            FormAction::Spud(b) => set(SPUD, Value::On(*b)),
            FormAction::Wings(b) => set(WINGS, Value::On(*b)),
            FormAction::Record(b) => set(RECORD, Value::On(*b)),
            FormAction::Start => Some(FieldAction::Start),
            FormAction::Cancel => Some(FieldAction::Cancel),
        }
    }

    /// Its words in a script's `form …` line: `level W1-2`, `lives 7`, `points 450000`,
    /// `energy 3`, `difficulty easy`, `spud on`, `wings off`, `record on`, `start`, `cancel`.
    /// `points` alone empties the field.
    pub fn named(words: &[&str]) -> Option<FormAction> {
        let on = |w: &str| match w {
            "on" => Some(true),
            "off" => Some(false),
            _ => None,
        };
        Some(match words {
            ["start"] => FormAction::Start,
            ["cancel"] => FormAction::Cancel,
            ["points", rest @ ..] => FormAction::Points(rest.join(" ")),
            ["level", id] => FormAction::Level(id.parse().ok().filter(|id| levels().contains(id))?),
            ["lives", n] => FormAction::Lives(*LIVES.iter().find(|l| l.count().to_string() == *n)?),
            ["energy", n] => FormAction::Energy(n.parse().ok().filter(|e| (1..=4).contains(e))?),
            ["difficulty", "easy"] => FormAction::Difficulty(Difficulty::Easy),
            ["difficulty", "normal"] => FormAction::Difficulty(Difficulty::Normal),
            ["spud", w] => FormAction::Spud(on(w)?),
            ["wings", w] => FormAction::Wings(on(w)?),
            ["record", w] => FormAction::Record(on(w)?),
            _ => return None,
        })
    }

    /// The words [`FormAction::named`] reads back.
    pub fn words(&self) -> String {
        let on = |b: bool| if b { "on" } else { "off" };
        match self {
            FormAction::Level(id) => format!("level {id}"),
            FormAction::Lives(l) => format!("lives {}", l.count()),
            FormAction::Points(t) => format!("points {t}").trim_end().to_string(),
            FormAction::Energy(e) => format!("energy {e}"),
            FormAction::Difficulty(d) => format!("difficulty {}", difficulty_title(*d).to_lowercase()),
            FormAction::Spud(b) => format!("spud {}", on(*b)),
            FormAction::Wings(b) => format!("wings {}", on(*b)),
            FormAction::Record(b) => format!("record {}", on(*b)),
            FormAction::Start => "start".into(),
            FormAction::Cancel => "cancel".into(),
        }
    }

    /// The actions that fill the window with `setup` and `record` from its defaults and press
    /// Start: a script's `start-level W1-2 lives=7 …` line, answered through the window's model.
    pub fn filling(setup: &LevelSetup, record: bool) -> Vec<FormAction> {
        vec![
            FormAction::Level(setup.level),
            FormAction::Lives(setup.lives),
            FormAction::Points(setup.points.to_string()),
            FormAction::Energy(setup.energy),
            FormAction::Difficulty(setup.difficulty),
            FormAction::Spud(setup.spud),
            FormAction::Wings(setup.wings),
            FormAction::Record(record),
            FormAction::Start,
        ]
    }
}

/// The Difficulty dropdown's title for `d`.
pub fn difficulty_title(d: Difficulty) -> &'static str {
    match d {
        Difficulty::Easy => "Easy",
        Difficulty::Normal => "Normal",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> LevelId {
        s.parse().unwrap()
    }

    #[test]
    fn the_window_opens_on_a_new_game_s_values_with_the_recorder_on() {
        let f = StartLevelForm::new();
        assert_eq!(f.setup(), LevelSetup::default());
        assert_eq!(f.setup().level, id("W1-1"));
        assert_eq!(f.setup().lives, Lives::Three);
        assert_eq!((f.setup().points, f.setup().energy), (0, 4));
        assert_eq!(f.setup().difficulty, Difficulty::Normal);
        assert!(!f.setup().spud && !f.setup().wings && f.record());
        assert_eq!(f.points_text(), "0");
        assert!(f.can_start());
        assert_eq!(f.points_hint(), "");
        let mut f = StartLevelForm::new();
        assert_eq!(f.apply(FormAction::Start), Some(Some((LevelSetup::default(), true))));
    }

    #[test]
    fn every_action_sets_its_value_and_start_returns_them_all() {
        let mut f = StartLevelForm::new();
        for a in [
            FormAction::Level(id("W1-2")),
            FormAction::Lives(Lives::Seven),
            FormAction::Points("450000".into()),
            FormAction::Energy(2),
            FormAction::Difficulty(Difficulty::Easy),
            FormAction::Spud(true),
            FormAction::Wings(true),
            FormAction::Record(false),
        ] {
            assert_eq!(f.apply(a.clone()), None, "{a:?} leaves the window open");
        }
        let want = LevelSetup {
            level: id("W1-2"),
            lives: Lives::Seven,
            points: 450_000,
            energy: 2,
            difficulty: Difficulty::Easy,
            spud: true,
            wings: true,
        };
        assert_eq!(f.setup(), want);
        assert_eq!(f.apply(FormAction::Start), Some(Some((want, false))));
        // Unticked again, the flags go back off.
        f.apply(FormAction::Spud(false));
        f.apply(FormAction::Wings(false));
        f.apply(FormAction::Record(true));
        assert!(!f.setup().spud && !f.setup().wings && f.record());
        assert_eq!(f.apply(FormAction::Cancel), Some(None));
    }

    #[test]
    fn points_out_of_range_or_not_a_whole_number_disable_start_and_show_the_range() {
        let mut f = StartLevelForm::new();
        for bad in ["1000000", "", " ", "-1", "+5", "1e3", "12.5", "450 000", "abc", "99999999999999999999"] {
            assert_eq!(f.apply(FormAction::Points(bad.into())), None);
            assert!(!f.can_start(), "{bad:?}");
            assert_eq!(f.points_hint(), POINTS_RANGE, "{bad:?}");
            assert_eq!(f.apply(FormAction::Start), None, "Start refused for {bad:?}");
        }
        assert_eq!(f.setup().points, 0, "a refused field keeps the last good points");
        for (good, p) in [("450000", 450_000), ("999999", 999_999), ("0", 0), (" 12 ", 12), ("007", 7)] {
            f.apply(FormAction::Points(good.into()));
            assert!(f.can_start(), "{good:?}");
            assert_eq!(f.points_hint(), "");
            assert_eq!(f.points(), Some(p));
        }
        f.apply(FormAction::Points("450000".into()));
        assert_eq!(f.apply(FormAction::Start).unwrap().unwrap().0.points, 450_000);
    }

    #[test]
    fn a_level_or_an_energy_the_dropdowns_do_not_offer_changes_nothing() {
        let mut f = StartLevelForm::new();
        assert_eq!(f.apply(FormAction::Level(id("W6-2alt"))), None);
        assert_eq!(f.apply(FormAction::Energy(0)), None);
        assert_eq!(f.apply(FormAction::Energy(5)), None);
        assert_eq!(f, StartLevelForm::new());
    }

    #[test]
    fn each_dropdown_offers_exactly_its_values_and_each_item_chooses_its_value() {
        let spec = spec();
        let titles = |i: usize| match &spec.fields[i].kind {
            FieldKind::Dropdown(t) => t.clone(),
            k => panic!("{k:?}"),
        };
        assert_eq!(titles(LEVEL).len(), 26, "the levels a game plays (T02, owner)");
        assert_eq!(titles(LEVEL)[0], "W1-1");
        assert_eq!(titles(LEVEL).last().unwrap(), "WW-1");
        assert!(!titles(LEVEL).iter().any(|t| t.contains("alt")));
        assert_eq!(titles(LIVES_FIELD), ["3", "5", "7"]);
        assert_eq!(titles(ENERGY), ["1", "2", "3", "4"]);
        assert_eq!(titles(DIFFICULTY), ["Easy", "Normal"]);
        let ids: Vec<&str> = spec.fields.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(ids, ["level", "lives", "points", "energy", "difficulty", "spud", "wings", "record"]);
        let labels: Vec<&str> = spec.fields.iter().map(|f| f.label.as_str()).collect();
        assert_eq!(labels, ["Level", "Lives", "Points", "Energy (hearts)", "Difficulty", "Spud", "Wings", "Enable recorder?"]);
        // Each item read back is the typed action that chooses it, and the defaults are what the
        // form opens on.
        let f = StartLevelForm::new();
        for (i, l) in levels().into_iter().enumerate() {
            assert_eq!(FormAction::Level(l).field(), Some(FieldAction::Set(LEVEL, Value::Choice(i))));
            let mut g = StartLevelForm::new();
            g.apply(FormAction::Level(l));
            assert_eq!(g.setup().level, l);
        }
        for (i, l) in LIVES.into_iter().enumerate() {
            assert_eq!(FormAction::Lives(l).field(), Some(FieldAction::Set(LIVES_FIELD, Value::Choice(i))));
        }
        for e in 1..=4u8 {
            let mut g = StartLevelForm::new();
            g.apply(FormAction::Energy(e));
            assert_eq!(g.setup().energy, e);
        }
        for (i, d) in DIFFICULTIES.into_iter().enumerate() {
            assert_eq!(FormAction::Difficulty(d).field(), Some(FieldAction::Set(DIFFICULTY, Value::Choice(i))));
        }
        for a in FormAction::filling(&LevelSetup::default(), true) {
            let mut g = StartLevelForm::new();
            if a != FormAction::Start {
                g.apply(a.clone());
                assert_eq!(g, f, "{a:?} is what it opens on");
            }
        }
    }

    #[test]
    fn a_script_names_each_action_and_reads_its_words_back() {
        let all = [
            FormAction::Level(id("WB-1")),
            FormAction::Lives(Lives::Five),
            FormAction::Points("1000000".into()),
            FormAction::Points(String::new()),
            FormAction::Energy(1),
            FormAction::Difficulty(Difficulty::Easy),
            FormAction::Difficulty(Difficulty::Normal),
            FormAction::Spud(true),
            FormAction::Wings(false),
            FormAction::Record(false),
            FormAction::Start,
            FormAction::Cancel,
        ];
        for a in all {
            let w = a.words();
            let words: Vec<&str> = w.split(' ').collect();
            assert_eq!(FormAction::named(&words), Some(a.clone()), "{w:?}");
        }
        for bad in [
            &["level", "W6-2alt"][..],
            &["level", "W7-1"],
            &["lives", "4"],
            &["energy", "0"],
            &["energy", "5"],
            &["difficulty", "hard"],
            &["spud", "yes"],
            &["record"],
            &["go"],
            &[],
        ] {
            assert_eq!(FormAction::named(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn filling_from_the_defaults_starts_with_exactly_the_setup() {
        let s = LevelSetup {
            level: id("W4-3"),
            lives: Lives::Five,
            points: 123_456,
            energy: 1,
            difficulty: Difficulty::Easy,
            spud: true,
            wings: false,
        };
        for record in [true, false] {
            let mut f = StartLevelForm::new();
            let mut out = None;
            for a in FormAction::filling(&s, record) {
                out = f.apply(a);
            }
            assert_eq!(out, Some(Some((s, record))));
        }
    }
}
