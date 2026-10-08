use serde::{Deserialize, Serialize};

/// One leg of the pomodoro cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Focus,
    ShortBreak,
    LongBreak,
}

impl Phase {
    pub fn is_break(self) -> bool {
        self != Phase::Focus
    }

    pub fn title(self) -> &'static str {
        match self {
            Phase::Focus => "Focus",
            Phase::ShortBreak => "Short Break",
            Phase::LongBreak => "Long Break",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_as_camel_case_strings() {
        assert_eq!(serde_json::to_string(&Phase::Focus).unwrap(), "\"focus\"");
        assert_eq!(serde_json::to_string(&Phase::ShortBreak).unwrap(), "\"shortBreak\"");
        assert_eq!(serde_json::to_string(&Phase::LongBreak).unwrap(), "\"longBreak\"");
    }

    #[test]
    fn titles_match_the_mac_app() {
        assert_eq!(Phase::Focus.title(), "Focus");
        assert_eq!(Phase::ShortBreak.title(), "Short Break");
        assert_eq!(Phase::LongBreak.title(), "Long Break");
    }

    #[test]
    fn only_focus_is_not_a_break() {
        assert!(!Phase::Focus.is_break());
        assert!(Phase::ShortBreak.is_break());
        assert!(Phase::LongBreak.is_break());
    }
}
