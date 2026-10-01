//! Tauri commands: the backend half of the frontend/backend API contract.
//!
//! Each command is a thin adapter: it receives a request from the UI, calls
//! application code, and returns a serializable DTO. Keep business logic out
//! of this module so it can be tested without the desktop app.
//! The contract is documented in docs/API.md.

pub mod app;
