//! Core: jobs, transactional outbox, event bus port, scheduler (doc 31)
//!
//! Notifications is a separate crate (`autotim-core-notifications`, doc 32)
//! built on top of the job queue exposed here — see doc 31
//! §"Notifications as Jobs" for the boundary.
//!
//! Status: scaffold only. Implements the `autotim_sdk::Module` trait
//! contract (see architecture doc 13 — Module System).

#![forbid(unsafe_code)]
