use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

#[derive(Debug, PartialEq)]
pub enum SyncStatus {
    Idle,
    Running,
}

pub struct SyncState {
    status: Mutex<SyncStatus>,
    pending: AtomicBool,
}

impl SyncState {
    pub fn new() -> Self {
        Self {
            status: Mutex::new(SyncStatus::Idle),
            pending: AtomicBool::new(false),
        }
    }

    /// Try to transition from Idle to Running. Returns true if successful.
    pub fn try_start(&self) -> bool {
        let mut status = self.status.lock().unwrap();
        if *status == SyncStatus::Idle {
            *status = SyncStatus::Running;
            true
        } else {
            false
        }
    }

    /// Set the pending flag (request a re-run after current sync finishes).
    pub fn set_pending(&self) {
        self.pending.store(true, Ordering::SeqCst);
    }

    /// Check and clear the pending flag. Returns true if a re-run was requested.
    pub fn take_pending(&self) -> bool {
        self.pending.swap(false, Ordering::SeqCst)
    }

    /// Transition back to Idle.
    pub fn set_idle(&self) {
        let mut status = self.status.lock().unwrap();
        *status = SyncStatus::Idle;
    }

    /// Check if currently running.
    pub fn is_running(&self) -> bool {
        let status = self.status.lock().unwrap();
        *status == SyncStatus::Running
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_start_from_idle() {
        let state = SyncState::new();
        assert!(state.try_start());
        assert!(state.is_running());
    }

    #[test]
    fn test_try_start_when_running_fails() {
        let state = SyncState::new();
        assert!(state.try_start());
        assert!(!state.try_start()); // already running
    }

    #[test]
    fn test_pending_flag() {
        let state = SyncState::new();
        assert!(!state.take_pending()); // initially false
        state.set_pending();
        assert!(state.take_pending()); // returns true and clears
        assert!(!state.take_pending()); // cleared
    }

    #[test]
    fn test_set_idle() {
        let state = SyncState::new();
        state.try_start();
        state.set_idle();
        assert!(!state.is_running());
        assert!(state.try_start()); // can start again
    }
}
