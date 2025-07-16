use rand::RngCore;
//use getrandom::getrandom;
use uuid::Uuid;

// use crate::prelude::*;

/// Produce a string containing a UUID.
///
/// Panics if random number generation fails.
pub fn uuid_str() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);

    let uuid = Uuid::new_v4();
    uuid.to_string()
}
