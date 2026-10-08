use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

/// Who pops out from behind the pill on a phase change. `None` means the whole
/// feature is off and no stage is ever built.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Character {
    Samurai,
    Ninja,
    Rabbit,
    AnimeGirl,
    General,
    Hancock,
    // Last because serde(other) must be. An unknown name in the file reads as
    // None, same as the Mac app.
    #[default]
    #[serde(other)]
    None,
}

/// User preferences, stored as one JSON file.
///
/// Missing keys fall back to the defaults; `clamped` pulls numbers into range.
/// A text box can hold a 0-minute or 10-hour pomodoro, and neither is intended.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub focus_minutes: i64,
    pub short_break_minutes: i64,
    pub long_break_minutes: i64,
    pub pomodoros_until_long_break: i64,
    pub auto_start_breaks: bool,
    pub auto_start_focus: bool,
    pub notifications_enabled: bool,
    pub chime_enabled: bool,
    /// 0 to 2, where 1.0 is the sample's own recorded level.
    pub chime_volume: f64,
    pub focus_chime: String,
    pub break_chime: String,
    pub control_music: bool,
    pub resume_music_on_focus_start: bool,
    pub show_floating_bar: bool,
    /// Shrink the bar to a bare progress ring while the timer runs.
    pub compact_floating_bar: bool,
    pub launch_at_login: bool,
    pub character: Character,
    /// None until the bar has been dragged, which means the default placement.
    pub floating_bar_x: Option<f64>,
    pub floating_bar_y: Option<f64>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            focus_minutes: 25,
            short_break_minutes: 5,
            long_break_minutes: 15,
            pomodoros_until_long_break: 4,
            auto_start_breaks: true,
            auto_start_focus: false,
            notifications_enabled: true,
            chime_enabled: true,
            chime_volume: 1.0,
            focus_chime: "Hero".into(),
            break_chime: "Blow".into(),
            control_music: true,
            resume_music_on_focus_start: true,
            show_floating_bar: true,
            compact_floating_bar: false,
            launch_at_login: false,
            character: Character::None,
            floating_bar_x: None,
            floating_bar_y: None,
        }
    }
}

impl Settings {
    /// A copy with every number inside its allowed range.
    pub fn clamped(mut self) -> Self {
        self.focus_minutes = self.focus_minutes.clamp(1, 240);
        self.short_break_minutes = self.short_break_minutes.clamp(1, 240);
        self.long_break_minutes = self.long_break_minutes.clamp(1, 240);
        self.pomodoros_until_long_break = self.pomodoros_until_long_break.clamp(1, 12);
        self.chime_volume = if self.chime_volume.is_nan() {
            1.0
        } else {
            self.chime_volume.clamp(0.0, 2.0)
        };
        self
    }

    /// Never fails: a missing or broken file gives the defaults.
    pub fn load(path: &Path) -> Settings {
        fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Settings>(&text).ok())
            .unwrap_or_default()
            .clamped()
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(&self.clone().clamped()).map_err(io::Error::other)?;
        fs::write(path, json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("pomodoro-settings-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir.join("settings.json")
    }

    #[test]
    fn defaults_match_the_mac_app() {
        let s = Settings::default();
        assert_eq!((s.focus_minutes, s.short_break_minutes, s.long_break_minutes), (25, 5, 15));
        assert_eq!(s.pomodoros_until_long_break, 4);
        assert!(s.auto_start_breaks && !s.auto_start_focus);
        assert!(s.notifications_enabled && s.chime_enabled);
        assert_eq!(s.chime_volume, 1.0);
        assert_eq!((s.focus_chime.as_str(), s.break_chime.as_str()), ("Hero", "Blow"));
        assert!(s.control_music && s.resume_music_on_focus_start);
        assert!(s.show_floating_bar && !s.compact_floating_bar && !s.launch_at_login);
        assert_eq!(s.character, Character::None);
        assert_eq!((s.floating_bar_x, s.floating_bar_y), (None, None));
    }

    #[test]
    fn minutes_clamp_to_1_through_240() {
        for (input, want) in [(-5, 1), (0, 1), (1, 1), (240, 240), (241, 240), (9999, 240)] {
            let s = Settings { focus_minutes: input, short_break_minutes: input, long_break_minutes: input, ..Default::default() }
                .clamped();
            assert_eq!((s.focus_minutes, s.short_break_minutes, s.long_break_minutes), (want, want, want));
        }
    }

    #[test]
    fn long_break_interval_clamps_to_1_through_12() {
        for (input, want) in [(-1, 1), (0, 1), (4, 4), (12, 12), (13, 12)] {
            let s = Settings { pomodoros_until_long_break: input, ..Default::default() }.clamped();
            assert_eq!(s.pomodoros_until_long_break, want);
        }
    }

    #[test]
    fn chime_volume_clamps_to_0_through_2() {
        for (input, want) in [(-1.0, 0.0), (0.0, 0.0), (1.5, 1.5), (2.0, 2.0), (3.0, 2.0)] {
            let s = Settings { chime_volume: input, ..Default::default() }.clamped();
            assert_eq!(s.chime_volume, want);
        }
        assert_eq!(Settings { chime_volume: f64::NAN, ..Default::default() }.clamped().chime_volume, 1.0);
    }

    #[test]
    fn missing_file_gives_defaults() {
        assert_eq!(Settings::load(&temp_path("missing")), Settings::default());
    }

    #[test]
    fn broken_file_gives_defaults() {
        let path = temp_path("broken");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "{ not json").unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
        fs::write(&path, "{\"focusMinutes\": \"lots\"}").unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
    }

    #[test]
    fn missing_and_unknown_keys_fall_back() {
        let path = temp_path("partial");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "{\"focusMinutes\": 50, \"somethingNew\": true}").unwrap();
        let s = Settings::load(&path);
        assert_eq!(s.focus_minutes, 50);
        assert_eq!(s.short_break_minutes, 5);
    }

    #[test]
    fn load_clamps_out_of_range_values() {
        let path = temp_path("clamp");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "{\"focusMinutes\": 0, \"pomodorosUntilLongBreak\": 99, \"chimeVolume\": 7}").unwrap();
        let s = Settings::load(&path);
        assert_eq!((s.focus_minutes, s.pomodoros_until_long_break, s.chime_volume), (1, 12, 2.0));
    }

    #[test]
    fn save_then_load_round_trips_and_creates_the_folder() {
        let path = temp_path("roundtrip");
        let s = Settings {
            focus_minutes: 40,
            character: Character::Hancock,
            floating_bar_x: Some(12.5),
            floating_bar_y: Some(80.0),
            ..Default::default()
        };
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path), s);
    }

    #[test]
    fn character_uses_the_mac_raw_values() {
        let names = [
            (Character::None, "none"), (Character::Samurai, "samurai"), (Character::Ninja, "ninja"),
            (Character::Rabbit, "rabbit"), (Character::AnimeGirl, "animeGirl"),
            (Character::General, "general"), (Character::Hancock, "hancock"),
        ];
        for (c, name) in names {
            assert_eq!(serde_json::to_string(&c).unwrap(), format!("\"{name}\""));
            assert_eq!(serde_json::from_str::<Character>(&format!("\"{name}\"")).unwrap(), c);
        }
    }

    #[test]
    fn unknown_character_reads_as_none() {
        let s: Settings = serde_json::from_str("{\"character\": \"dragon\"}").unwrap();
        assert_eq!(s.character, Character::None);
    }

    #[test]
    fn keys_are_camel_case() {
        let json = serde_json::to_value(Settings::default()).unwrap();
        for key in ["focusMinutes", "pomodorosUntilLongBreak", "resumeMusicOnFocusStart", "compactFloatingBar", "chimeVolume"] {
            assert!(json.get(key).is_some(), "missing {key}");
        }
    }
}
