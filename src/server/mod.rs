//! The login server (docs/ONLINE_PLAN.md, phase 3): accounts, heroes kept on the server, and games anyone
//! logged in can create or join. `ashensanctum --server` runs it with no window (Docker: see docker/).
#![forbid(unsafe_code)]

pub mod accounts;
