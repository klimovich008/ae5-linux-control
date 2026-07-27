//! Structured interaction trace, off by default.
//!
//! Every fault class this application has actually hit was invisible in
//! hindsight: a full-window rebuild storm triggered by its own mixer writes, a
//! route switch racing a rebuild, and a DSP fault with no record of the writes
//! that preceded it. The journal knew nothing because the application logged
//! nothing.
//!
//! `AE5_TRACE=1` turns on a monotonic, single-line trace on stderr:
//!
//! ```text
//! [ae5 +12.041s mixer] write FX: X-Bass = on -> verified 53 [on]
//! [ae5 +12.043s watch] self-originated event suppressed (34 ms after write)
//! [ae5 +14.987s refresh] rebuild: reason=external-event page=effects
//! ```
//!
//! The format is deliberately grep-shaped rather than pretty. Redirect stderr
//! to a file when reporting a fault and the sequence of cause and effect is in
//! the report.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

static START: OnceLock<Instant> = OnceLock::new();
static ENABLED: OnceLock<bool> = OnceLock::new();
/// Milliseconds since [`START`] of the most recent write this process made.
static LAST_SELF_WRITE_MS: AtomicU64 = AtomicU64::new(u64::MAX);

/// How long after our own mixer write an ALSA event is still assumed to be the
/// echo of that write. Scale drags emit bursts well inside this window; a
/// change from another process arrives outside it in practice.
pub const SELF_EVENT_WINDOW_MS: u64 = 400;

fn start() -> Instant {
    *START.get_or_init(Instant::now)
}

/// Whether tracing was requested via `AE5_TRACE`.
pub fn enabled() -> bool {
    *ENABLED.get_or_init(|| std::env::var_os("AE5_TRACE").is_some_and(|value| value != "0"))
}

fn elapsed_ms() -> u64 {
    start().elapsed().as_millis() as u64
}

/// Emit one trace line. Cheap no-op unless `AE5_TRACE` is set.
pub fn trace(area: &str, message: &str) {
    if !enabled() {
        return;
    }
    eprintln!(
        "[ae5 +{:>9.3}s {area}] {message}",
        start().elapsed().as_secs_f64()
    );
}

/// Record that this process just wrote to the mixer.
///
/// The watch thread uses this to tell an echo of our own write apart from a
/// genuinely external change, instead of rebuilding the window for both.
pub fn note_self_write() {
    LAST_SELF_WRITE_MS.store(elapsed_ms(), Ordering::Release);
}

/// If the event now being handled is within the self-event window, return how
/// many milliseconds after our write it arrived; otherwise `None`.
pub fn self_event_age_ms() -> Option<u64> {
    let last = LAST_SELF_WRITE_MS.load(Ordering::Acquire);
    if last == u64::MAX {
        return None;
    }
    let age = elapsed_ms().saturating_sub(last);
    (age < SELF_EVENT_WINDOW_MS).then_some(age)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_write_marks_a_bounded_window() {
        assert_eq!(
            self_event_age_ms(),
            None,
            "no write recorded yet, nothing may be suppressed"
        );
        note_self_write();
        let age = self_event_age_ms().expect("a just-recorded write is inside the window");
        assert!(age < SELF_EVENT_WINDOW_MS);
    }
}
