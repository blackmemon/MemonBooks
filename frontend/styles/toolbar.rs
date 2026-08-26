//! Toolbar row layout constant (Section 6) -- the shared height every
//! control in a list-view toolbar row (search input, buttons,
//! pick_lists) lines up against. The bordered dropdown-button look
//! toolbar controls use is `styles::button::secondary` now (Batch 3 of
//! the Section 6 consistency audit consolidated it there, since it was
//! byte-identical to `bulk_action_bar::secondary_button` and one branch
//! of `src/main.rs`'s `transaction_side_button`).

pub const CONTROL_HEIGHT: f32 = 36.0;
