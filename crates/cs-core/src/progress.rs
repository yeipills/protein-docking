//! Progress reporting and cooperative cancellation for long computations.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::{Error, Result};

/// Receives progress updates from a pipeline. Implementations must be cheap and thread-safe:
/// they are called from rayon worker threads.
pub trait Progress: Sync {
    /// `stage` is a short machine-friendly name, `fraction` is in `[0, 1]` for the whole run.
    fn report(&self, stage: &str, fraction: f32);

    /// Returns true when the caller asked to stop. Checked between chunks of work.
    fn is_cancelled(&self) -> bool {
        false
    }
}

/// Progress sink that ignores everything.
pub struct NoProgress;

impl Progress for NoProgress {
    fn report(&self, _stage: &str, _fraction: f32) {}
}

/// Progress sink backed by a cancellation flag and a callback.
pub struct FnProgress<'a, F: Fn(&str, f32) + Sync> {
    pub callback: F,
    pub cancel: &'a AtomicBool,
}

impl<F: Fn(&str, f32) + Sync> Progress for FnProgress<'_, F> {
    fn report(&self, stage: &str, fraction: f32) {
        (self.callback)(stage, fraction);
    }

    fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}

pub(crate) fn check(progress: &dyn Progress) -> Result<()> {
    if progress.is_cancelled() {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
