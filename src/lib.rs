// Crate root. backend/, configs/, database/ and frontend/ live outside
// src/ per the frozen structure in PROJECT_SPEC.md Section 2, so they are
// wired in with explicit #[path] attributes instead of Cargo's default
// src/-relative module discovery.

#[path = "../backend/mod.rs"]
pub mod backend;

#[path = "../configs/mod.rs"]
pub mod configs;

#[path = "../database/mod.rs"]
pub mod database;

#[path = "../frontend/mod.rs"]
pub mod frontend;
