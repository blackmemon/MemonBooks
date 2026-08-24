//! A collision-safe, auto-generated reference code shared by any entity
//! that wants one: `<PREFIX>-YYYYMMDDHHMMSSmmm` (UTC, millisecond
//! precision) -- e.g. `COA-20260824071523482`.
//!
//! Backed by an in-process counter, not a DB unique-constraint-and-
//! retry loop: every repository in this codebase does a single INSERT
//! with no retry plumbing anywhere, so generating an already-unique
//! value up front fits that pattern far better than adding retry logic
//! to every call site. Two calls landing in the same millisecond (bulk
//! import, rapid clicks) get a `-N` suffix appended, so the plain,
//! suffix-free form in the example above stays the common case.
//!
//! The counter is keyed per exact millisecond in a `Mutex<HashMap>`,
//! not a single "current epoch" pointer -- an earlier version tracked
//! just the most recent millisecond and merged any call for an older
//! millisecond into whatever epoch was currently live, on the theory
//! that the counter only needs to be unique, not tied to a specific
//! timestamp. That's wrong: a thread can read the clock and then be
//! preempted before it claims a slot, so two different threads that
//! independently read the *same* (coarse-resolution) millisecond can
//! be merged into two *different* later epochs and coincidentally land
//! on the same relative offset in each -- same displayed timestamp,
//! same counter, same string. A per-millisecond map has no such
//! coupling: two calls only ever share a counter chain if they share
//! the exact millisecond they'll actually display, so the string they
//! produce can never collide. Caught by
//! `concurrent_generation_from_multiple_threads_is_still_collision_free`
//! before this fix.
//!
//! This only guarantees uniqueness within one running process -- fine
//! for a single-instance desktop app; callers that also want a DB-level
//! backstop should still give the column a `UNIQUE` index, same as
//! every other generated/user-entered code column in this codebase.
//! The map grows by one entry per distinct (prefix, millisecond) pair
//! that has ever had a code generated in this process, which is
//! negligible: codes are only generated on entity creation (a user
//! action), never in a hot loop.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use chrono::Utc;

// Keyed by (prefix, millisecond), not millisecond alone: different
// prefixes can never collide with each other regardless of counter (the
// prefix itself disambiguates them), so there's no reason for a
// Customer created in the same millisecond as a Chart of Accounts entry
// to consume a counter slot the other one just as validly wants -- that
// only makes the "usually no suffix" common case less common than it
// needs to be.
fn counters() -> &'static Mutex<HashMap<(String, i64), i64>> {
    static COUNTERS: OnceLock<Mutex<HashMap<(String, i64), i64>>> = OnceLock::new();
    COUNTERS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Generates a reference code with the given prefix, e.g.
/// `generate_code("COA")` -> `"COA-20260824071523482"`.
pub fn generate_code(prefix: &str) -> String {
    let now = Utc::now();
    let timestamp = now.format("%Y%m%d%H%M%S%3f");

    let mut counters = counters().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let slot = counters.entry((prefix.to_string(), now.timestamp_millis())).or_insert(0);
    let counter = *slot;
    *slot += 1;
    drop(counters);

    if counter == 0 {
        format!("{prefix}-{timestamp}")
    } else {
        format!("{prefix}-{timestamp}-{counter}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn generated_code_has_the_expected_shape() {
        let code = generate_code("COA");
        assert!(code.starts_with("COA-"));

        // The common case is a bare 17-digit YYYYMMDDHHMMSSmmm with no
        // suffix, but this test runs concurrently with the rest of the
        // suite -- including plenty of other `generate_code("COA", ...)`
        // calls from unrelated tests via `Account::new` -- so a shared
        // millisecond legitimately produces a "-N" suffix here too. Both
        // shapes are valid; what matters is the timestamp part is always
        // exactly 17 digits and any suffix is always digits-only.
        let rest = &code["COA-".len()..];
        let mut parts = rest.splitn(2, '-');
        let timestamp_part = parts.next().unwrap();
        assert_eq!(timestamp_part.len(), 17);
        assert!(timestamp_part.chars().all(|c| c.is_ascii_digit()));
        if let Some(suffix) = parts.next() {
            assert!(!suffix.is_empty());
            assert!(suffix.chars().all(|c| c.is_ascii_digit()));
        }
    }

    #[test]
    fn different_prefixes_are_respected() {
        assert!(generate_code("CUST").starts_with("CUST-"));
        assert!(generate_code("VEND").starts_with("VEND-"));
    }

    #[test]
    fn many_codes_generated_in_a_tight_loop_are_all_unique() {
        // Proves the same-millisecond collision guarantee actually
        // works, not just that it compiles: thousands of calls in a
        // tight loop will land in the same millisecond many times over.
        let codes: HashSet<String> = (0..10_000).map(|_| generate_code("TEST")).collect();
        assert_eq!(codes.len(), 10_000);
    }

    #[test]
    fn concurrent_generation_from_multiple_threads_is_still_collision_free() {
        use std::thread;

        let handles: Vec<_> = (0..8).map(|_| thread::spawn(|| (0..2_000).map(|_| generate_code("MT")).collect::<Vec<_>>())).collect();

        let mut all = HashSet::new();
        for handle in handles {
            for code in handle.join().unwrap() {
                assert!(all.insert(code), "duplicate code generated across threads");
            }
        }
        assert_eq!(all.len(), 16_000);
    }
}
