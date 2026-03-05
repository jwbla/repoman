#![warn(clippy::pedantic)]
#![allow(dead_code)] // lib.rs re-exports for integration tests; many items only used from binary
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::format_push_string)]
#![allow(clippy::manual_let_else)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::match_same_arms)]

pub mod config;
pub mod error;
pub mod metadata;
pub mod operations;
pub mod vault;

// Internal modules needed by operations but not public API
pub(crate) mod hooks;
pub(crate) mod plugins;
pub(crate) mod util;
