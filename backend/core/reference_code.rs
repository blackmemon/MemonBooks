use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use chrono::Utc;

fn counters() -> &'static Mutex<HashMap<(String, i64), i64>> {
    static COUNTERS: OnceLock<Mutex<HashMap<(String, i64), i64>>> = OnceLock::new();
    COUNTERS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// `generate_code("COA")` -> `"COA-20260824071523482"`.
pub fn generate_code(prefix: &str) -> String {
    let now = Utc::now();
    let timestamp = now.format("%Y%m%d%H%M%S%3f");

    let mut counters = counters()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let slot = counters
        .entry((prefix.to_string(), now.timestamp_millis()))
        .or_insert(0);
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
        let codes: HashSet<String> = (0..10_000).map(|_| generate_code("TEST")).collect();
        assert_eq!(codes.len(), 10_000);
    }

    #[test]
    fn concurrent_generation_from_multiple_threads_is_still_collision_free() {
        use std::thread;

        let handles: Vec<_> = (0..8)
            .map(|_| thread::spawn(|| (0..2_000).map(|_| generate_code("MT")).collect::<Vec<_>>()))
            .collect();

        let mut all = HashSet::new();
        for handle in handles {
            for code in handle.join().unwrap() {
                assert!(all.insert(code), "duplicate code generated across threads");
            }
        }
        assert_eq!(all.len(), 16_000);
    }
}
