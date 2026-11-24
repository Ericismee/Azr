use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdleHide {
    Keep,
    Wait(Duration),
    Hide,
}

#[derive(Default)]
pub struct IdleTimer {
    since: Option<Instant>,
}

impl IdleTimer {
    pub fn update(&mut self, now: Instant, delay: Duration, interacting: bool) -> IdleHide {
        if interacting {
            self.since = None;
            return IdleHide::Keep;
        }
        let elapsed = now.saturating_duration_since(*self.since.get_or_insert(now));
        if elapsed >= delay {
            IdleHide::Hide
        } else {
            IdleHide::Wait(delay - elapsed)
        }
    }
}

pub struct RevealZone {
    waiting_for_exit: bool,
    entered_at: Option<Instant>,
}

impl RevealZone {
    pub fn new(inside: bool) -> Self {
        Self {
            waiting_for_exit: inside,
            entered_at: None,
        }
    }

    pub fn should_reveal(&mut self, now: Instant, inside: bool) -> bool {
        if !inside {
            self.waiting_for_exit = false;
            self.entered_at = None;
            return false;
        }
        if self.waiting_for_exit {
            return false;
        }
        let entered = *self.entered_at.get_or_insert(now);
        if now.saturating_duration_since(entered) < Duration::from_millis(110) {
            return false;
        }
        self.waiting_for_exit = true;
        true
    }
}

pub fn contains(bounds: (i32, i32, i32, i32), point: (i32, i32)) -> bool {
    let (left, top, right, bottom) = bounds;
    (left..right).contains(&point.0) && (top..bottom).contains(&point.1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use IdleHide::*;

    #[test]
    fn reveal_requires_a_new_hover_at_the_edge() {
        let start = Instant::now();
        let at = |ms| start + Duration::from_millis(ms);
        let mut edge = RevealZone::new(true);
        assert!(!edge.should_reveal(at(200), true));
        assert!(!edge.should_reveal(at(210), false));
        assert!(!edge.should_reveal(at(220), true));
        assert!(!edge.should_reveal(at(329), true));
        assert!(edge.should_reveal(at(330), true));
        assert!(!edge.should_reveal(at(500), true));
    }

    #[test]
    fn grazing_the_bottom_edge_does_not_reveal_the_dock() {
        let start = Instant::now();
        let at = |ms| start + Duration::from_millis(ms);
        let mut edge = RevealZone::new(false);
        assert!(!edge.should_reveal(at(0), true));
        assert!(!edge.should_reveal(at(80), false));
        assert!(!edge.should_reveal(at(200), true));
        assert!(!edge.should_reveal(at(300), true));
        assert!(edge.should_reveal(at(310), true));
    }

    #[test]
    fn bottom_edge_excludes_app_controls_above_it_on_a_secondary_display() {
        let bounds = (-840, 1066, -260, 1080);
        assert!(!contains(bounds, (-550, 1040)));
        assert!(!contains(bounds, (-550, 1065)));
        assert!(contains(bounds, (-550, 1066)));
        assert!(contains(bounds, (-550, 1079)));
        assert!(!contains(bounds, (-850, 1079)));
        assert!(!contains(bounds, (-550, 1080)));
    }

    #[test]
    fn waits_for_the_configured_delay_without_resetting_on_refresh() {
        let start = Instant::now();
        for seconds in [3, 5, 10] {
            let delay = Duration::from_secs(seconds);
            let mut idle = IdleTimer::default();
            assert_eq!(idle.update(start, delay, false), Wait(delay));
            assert_eq!(
                idle.update(start + delay - Duration::from_millis(1), delay, false),
                Wait(Duration::from_millis(1))
            );
            assert_eq!(idle.update(start + delay, delay, false), Hide);
        }
    }

    #[test]
    fn entering_dock_or_preview_cancels_the_pending_hide() {
        let start = Instant::now();
        let delay = Duration::from_secs(3);
        let mut idle = IdleTimer::default();
        assert_eq!(idle.update(start, delay, false), Wait(delay));
        assert_eq!(
            idle.update(start + Duration::from_secs(2), delay, true),
            Keep
        );
        assert_eq!(idle.update(start + delay, delay, false), Wait(delay));
        assert_eq!(idle.update(start + delay * 2, delay, false), Hide);
    }

    #[test]
    fn changing_delay_uses_the_original_leave_time() {
        let start = Instant::now();
        let mut idle = IdleTimer::default();
        assert_eq!(
            idle.update(start, Duration::from_secs(10), false),
            Wait(Duration::from_secs(10))
        );
        assert_eq!(
            idle.update(
                start + Duration::from_secs(4),
                Duration::from_secs(3),
                false
            ),
            Hide
        );
    }
}
