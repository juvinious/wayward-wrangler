//! Paired local IPC budgets. Large read-only state does not expand mutation input.
pub const REQUEST_FRAME_MAX: usize = 1024 * 1024;
pub const RESPONSE_FRAME_MAX: usize = 8 * 1024 * 1024;
pub const COMMAND_MAX: usize = 64 * 1024;
