use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub focus_ms: u64,
    pub break_ms: u64,
}
#[derive(Serialize, Deserialize)]
pub struct Data {
    pub version: u32,
    pub projects: Vec<Project>,
    pub include_breaks: bool,
    pub pomodoro: bool,
}
impl Default for Data {
    fn default() -> Self {
        Self {
            version: 1,
            projects: vec![],
            include_breaks: false,
            pomodoro: false,
        }
    }
}
#[derive(Default)]
pub struct Timer {
    pub running: bool,
    pub on_break: bool,
    pub phase_ms: u64,
    pub session_ms: u64,
    pub rounds: u64,
}
impl Timer {
    pub fn limit(&self) -> u64 {
        if !self.on_break {
            25 * 60_000
        } else if self.rounds.is_multiple_of(4) {
            15 * 60_000
        } else {
            5 * 60_000
        }
    }
    pub fn advance(&mut self, mut ms: u64, project: &mut Project, pomodoro: bool) {
        if !self.running {
            return;
        }
        self.session_ms += ms;
        while ms > 0 {
            let step = if pomodoro {
                ms.min(self.limit() - self.phase_ms)
            } else {
                ms
            };
            if pomodoro && self.on_break {
                project.break_ms += step;
            } else {
                project.focus_ms += step;
            }
            self.phase_ms += step;
            ms -= step;
            if pomodoro && self.phase_ms == self.limit() {
                if !self.on_break {
                    self.rounds += 1;
                }
                self.on_break = !self.on_break;
                self.phase_ms = 0;
            }
        }
    }
}
pub fn duration(ms: u64) -> String {
    let s = ms / 1000;
    format!("{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stopwatch_and_pause() {
        let mut t = Timer {
            running: true,
            ..Default::default()
        };
        let mut p = Project::default();
        t.advance(3456, &mut p, false);
        t.running = false;
        t.advance(5000, &mut p, false);
        assert_eq!(p.focus_ms, 3456);
        assert_eq!(p.break_ms, 0);
    }
    #[test]
    fn delayed_tick_splits_focus_and_break() {
        let mut t = Timer {
            running: true,
            ..Default::default()
        };
        let mut p = Project::default();
        t.advance(31 * 60_000, &mut p, true);
        assert_eq!(p.focus_ms, 26 * 60_000);
        assert_eq!(p.break_ms, 5 * 60_000);
        assert!(!t.on_break);
        assert_eq!(t.phase_ms, 60_000);
    }
    #[test]
    fn fourth_round_has_long_break() {
        let mut t = Timer {
            running: true,
            ..Default::default()
        };
        let mut p = Project::default();
        t.advance(115 * 60_000, &mut p, true);
        assert_eq!(t.rounds, 4);
        assert!(t.on_break);
        assert_eq!(t.limit(), 15 * 60_000);
        t.advance(15 * 60_000, &mut p, true);
        assert_eq!(p.focus_ms, 100 * 60_000);
        assert_eq!(p.break_ms, 30 * 60_000);
        assert!(!t.on_break);
    }
}
