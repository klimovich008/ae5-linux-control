//! Structured interaction trace, on by default during pre-release validation.
//!
//! Every fault class this application has actually hit was invisible in
//! hindsight: a full-window rebuild storm triggered by its own mixer writes, a
//! route switch racing a rebuild, and a DSP fault with no record of the writes
//! that preceded it. The journal knew nothing because the application logged
//! nothing.
//!
//! The monotonic, single-line trace is written to stderr so the desktop
//! session journal can retain and rotate it:
//!
//! ```text
//! [ae5 +12.041s mixer] write FX: X-Bass = on -> verified 53 [on]
//! [ae5 +12.043s watch] self-originated event suppressed (34 ms after write)
//! [ae5 +14.987s refresh] rebuild: reason=external-event page=effects
//! ```
//!
//! The format is deliberately grep-shaped rather than pretty. Set
//! `AE5_TRACE=0` to opt out.

use std::ffi::OsStr;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

static START: OnceLock<Instant> = OnceLock::new();
static ENABLED: OnceLock<bool> = OnceLock::new();
/// Milliseconds since [`START`] of the most recent write this process made.
static LAST_SELF_WRITE_MS: AtomicU64 = AtomicU64::new(u64::MAX);
static SELF_WRITE_GENERATION: AtomicU64 = AtomicU64::new(0);

/// How long after our own mixer write an ALSA event is still assumed to be the
/// echo of that write. Scale drags emit bursts well inside this window; a
/// change from another process arrives outside it in practice.
pub const SELF_EVENT_WINDOW_MS: u64 = 400;

fn start() -> Instant {
    *START.get_or_init(Instant::now)
}

fn enabled_from_env(value: Option<&OsStr>) -> bool {
    value != Some(OsStr::new("0"))
}

/// Whether tracing is enabled. `AE5_TRACE=0` is the explicit opt-out.
pub fn enabled() -> bool {
    *ENABLED.get_or_init(|| enabled_from_env(std::env::var_os("AE5_TRACE").as_deref()))
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
    SELF_WRITE_GENERATION.fetch_add(1, Ordering::AcqRel);
    LAST_SELF_WRITE_MS.store(elapsed_ms(), Ordering::Release);
}

/// Number of mixer or runtime-state writes made by this process.
pub fn self_write_generation() -> u64 {
    SELF_WRITE_GENERATION.load(Ordering::Acquire)
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
    fn tracing_defaults_on_and_has_an_explicit_opt_out() {
        assert!(enabled_from_env(None));
        assert!(enabled_from_env(Some(OsStr::new("1"))));
        assert!(!enabled_from_env(Some(OsStr::new("0"))));
    }

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
