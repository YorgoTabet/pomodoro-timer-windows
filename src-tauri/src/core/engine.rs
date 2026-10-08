use super::{phase::Phase, settings::Settings};

/// The phase that follows `phase`.
///
/// `completed_focus_sessions` counts sessions finished *before* this phase. When
/// `phase` is focus, finishing it makes the count one higher, and that new total
/// decides whether the upcoming break is long.
pub fn next(phase: Phase, completed_focus_sessions: u32, pomodoros_until_long_break: u32) -> Phase {
    if phase != Phase::Focus {
        return Phase::Focus;
    }
    let interval = pomodoros_until_long_break.max(1);
    if (completed_focus_sessions + 1).is_multiple_of(interval) {
        Phase::LongBreak
    } else {
        Phase::ShortBreak
    }
}

/// Seconds a phase should run for, under the given settings.
pub fn duration(phase: Phase, settings: &Settings) -> i64 {
    let minutes = match phase {
        Phase::Focus => settings.focus_minutes,
        Phase::ShortBreak => settings.short_break_minutes,
        Phase::LongBreak => settings.long_break_minutes,
    };
    minutes * 60
}

/// Whether the phase that just ended should be followed by the next one on its own.
pub fn should_auto_start(after: Phase, settings: &Settings) -> bool {
    // Finishing focus leads into a break; finishing a break leads into focus.
    if after == Phase::Focus {
        settings.auto_start_breaks
    } else {
        settings.auto_start_focus
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breaks_always_lead_back_to_focus() {
        assert_eq!(next(Phase::ShortBreak, 3, 4), Phase::Focus);
        assert_eq!(next(Phase::LongBreak, 4, 4), Phase::Focus);
    }

    #[test]
    fn every_fourth_focus_gets_a_long_break() {
        let phases: Vec<Phase> = (0..8).map(|done| next(Phase::Focus, done, 4)).collect();
        use Phase::*;
        assert_eq!(phases, [ShortBreak, ShortBreak, ShortBreak, LongBreak, ShortBreak, ShortBreak, ShortBreak, LongBreak]);
    }

    #[test]
    fn interval_of_one_makes_every_break_long() {
        assert_eq!(next(Phase::Focus, 0, 1), Phase::LongBreak);
        assert_eq!(next(Phase::Focus, 5, 1), Phase::LongBreak);
    }

    #[test]
    fn interval_of_zero_is_treated_as_one() {
        assert_eq!(next(Phase::Focus, 0, 0), Phase::LongBreak);
    }

    #[test]
    fn durations_follow_the_settings() {
        let s = Settings { focus_minutes: 30, short_break_minutes: 7, long_break_minutes: 20, ..Default::default() };
        assert_eq!(duration(Phase::Focus, &s), 1800);
        assert_eq!(duration(Phase::ShortBreak, &s), 420);
        assert_eq!(duration(Phase::LongBreak, &s), 1200);
        assert_eq!(duration(Phase::Focus, &Settings::default()), 1500);
    }

    #[test]
    fn auto_start_uses_breaks_flag_after_focus_and_focus_flag_after_a_break() {
        let s = Settings { auto_start_breaks: true, auto_start_focus: false, ..Default::default() };
        assert!(should_auto_start(Phase::Focus, &s));
        assert!(!should_auto_start(Phase::ShortBreak, &s));
        assert!(!should_auto_start(Phase::LongBreak, &s));
        let s = Settings { auto_start_breaks: false, auto_start_focus: true, ..Default::default() };
        assert!(!should_auto_start(Phase::Focus, &s));
        assert!(should_auto_start(Phase::ShortBreak, &s));
        assert!(should_auto_start(Phase::LongBreak, &s));
    }
}
