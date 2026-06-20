//! Core: Notifications (doc 32).
//!
//! A thin Core module (per doc 00's Core boundary note) that resolves
//! templates, recipients, and eligible channels, then enqueues a
//! `notify.deliver` job per (recipient, channel) on the Core job queue
//! (`autotim-core-async`, doc 31). Channel implementations are
//! Integration Providers, following the same capability-aware pattern
//! as doc 42 (Integration Provider Contracts) — they live in `providers/`
//! and are never exposed as separate Module Registry entries.

#![forbid(unsafe_code)]

pub mod provider;
pub mod providers;

pub use provider::{
    ChannelCapabilities, ChannelRecipient, DeliveryReceipt, NotificationChannelProvider,
    RenderedNotification,
};
