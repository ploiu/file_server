use std::sync::{Arc, Mutex, TryLockError};
use std::time::Instant;

use rocket::State;

pub fn update_last_request_time(last_request_time: &State<Arc<Mutex<Instant>>>) {
    match last_request_time.try_lock() {
        Ok(mut lock) => *lock = Instant::now(),
        Err(TryLockError::Poisoned(mut e)) => {
            log::warn!("The last_request_time mutex was poisoned! Resetting...");
            **e.get_mut() = Instant::now();
            last_request_time.clear_poison();
        }
        Err(TryLockError::WouldBlock) => { /*no op*/ }
    }
}
