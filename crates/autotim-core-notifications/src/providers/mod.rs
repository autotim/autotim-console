//! Reference channel providers (doc 32 §"Channels Are Providers, Not Core Code").
//!
//! Status: scaffold. Each channel is implemented as its own
//! `NotificationChannelProvider`, selected per organization via Settings
//! (doc 30) with credentials resolved through Secrets (doc 23) — never
//! hardcoded here.
//!
//! Planned: email (SMTP), in_app, webhook, telegram, discord, matrix, push.

// pub mod email;
// pub mod in_app;
// pub mod webhook;
// pub mod telegram;
// pub mod discord;
// pub mod matrix;
// pub mod push;
