# 32 — Notifications Architecture

## Purpose

Centralized notification delivery across the platform. Doc 31 (Async Substrate) establishes the correct foundation — notifications are **a job type**, not a third independent queue/retry/DLQ stack. This document specifies the notification domain itself: channels, templates, preferences, and priority — and corrects a v2 inconsistency by treating delivery channels as **Integration Providers**, exactly the pattern doc 42 already established for DNS, SSL, Mail, Monitoring, and Network.

Relationship to doc 31: a `notify.deliver` job is enqueued on the same Core job queue as everything else (doc 31 §"Notifications as Jobs"). This document does not redefine retry/backoff/DLQ — it defines what the job *does* once a worker picks it up.

## Channels Are Providers, Not Core Code

The v2 first draft described channels ("Email, Telegram, Discord, Matrix, Webhook, Push") as ad-hoc "channel adapters" baked into Core. That contradicts the Integration Provider pattern already established for every other external-system integration (doc 42). Corrected model:

```text
Notifications Module (Core, thin)
└── internal provider registry (doc 42 §2 pattern)
    ├── EmailProvider (SMTP)
    ├── InAppProvider
    ├── WebhookProvider
    ├── TelegramProvider
    ├── DiscordProvider
    ├── MatrixProvider
    └── PushProvider (Web Push / mobile, doc 50 PWA)
```

This keeps Core's surface thin (doc 00 — Core boundary note) and lets a community add a new channel (e.g. Slack, ntfy.sh) as a new provider, never as a Core code change.

## `NotificationChannelProvider` Contract

Same shape as every other contract in doc 42 — capability-aware, with the shared `ProviderError`/`ProviderHealth` types:

```rust
pub struct ChannelCapabilities {
    pub supports_rich_formatting: bool,
    pub supports_delivery_receipts: bool,
    pub max_message_length: Option<usize>,
}

#[async_trait]
pub trait NotificationChannelProvider: Send + Sync {
    fn capabilities(&self) -> ChannelCapabilities;

    async fn send(
        &self,
        recipient: &ChannelRecipient,
        rendered: &RenderedNotification,
    ) -> Result<DeliveryReceipt, ProviderError>;

    async fn health(&self) -> ProviderHealth;
}
```

A channel a user has not configured (e.g. no Telegram chat ID linked) is simply absent from that user's enabled-channels list — not a `ProviderError`; that distinction matters for §"User Preferences" below.

## Credentials & Per-Organization Configuration

Exactly the doc 30 / doc 42 §3 pattern: the active channel set and any channel credentials (SMTP host/password, Telegram bot token, Discord webhook URL) are Settings entries of type `secret` where sensitive, scoped per organization. No channel provider ever receives a raw credential except through `SecretStore` (doc 23), resolved at send time.

## Notification Flow

```text
Module/event triggers a notification
   → Notifications module resolves: template, recipient(s), eligible
     channels (per user preference + org configuration)
   → enqueue notify.deliver job per (recipient, channel) — doc 31
   → worker renders the template, calls the channel's
     NotificationChannelProvider::send()
   → DeliveryReceipt recorded; status updated
```

Delivery status: `pending → sent → delivered | failed`, identical vocabulary to the v1 design, now persisted as job/job_run state (doc 31) rather than a separate table family.

## Templates

Templates support variables, localization (doc 50 — `vue-i18n` on the frontend; server-side templates use the same locale keys), and branding (organization name/logo where relevant — EE white-labeling hook). Example:

```text
template: certificate_expiring
  vars: { hostname, expiration_date, days_remaining }
  locales: en, ro, de, ...
```

A template is rendered once per recipient locale, then handed to each eligible channel's provider — rich channels (Discord, Matrix) may receive a richer rendering than plain-text channels (SMS-like webhook) when `supports_rich_formatting` is false.

## User Preferences

Users configure (Settings, User scope, doc 30): enabled channels, notification categories (e.g. security, infrastructure, billing — opt out of categories, not just channels), and **quiet hours** (a time window where only `Critical` severity bypasses suppression — see below). Preferences are read once per notification dispatch and cached for the duration of that dispatch only (not long-lived, to avoid stale-preference bugs after a user just changed settings).

## Security Notification Priority

Certain categories are **not fully suppressible** by user preference, consistent with doc 20's security posture: MFA disabled, suspicious login, privilege escalation, secret access anomalies. These always deliver to at least one channel (defaulting to email/in-app if the user has disabled everything else) and ignore quiet hours when severity is `Critical`. This is a deliberate override of user preference for the user's own protection — documented here so it is not mistaken for a notification-preferences bug later.

## Delivery Status & Retry

Fully inherited from doc 31's job retry/backoff/DLQ — not duplicated here. A `ProviderError::Unreachable` (e.g. SMTP server down) retries with backoff; `ProviderError::Unauthenticated` (bad credentials) fails fast and raises an operator-facing notification through the `InAppProvider` (the one channel assumed always configured) so a broken integration doesn't fail silently — same `ProviderError` mapping pattern as doc 42 §7.

## Audit

Every notification dispatch is audited at the category level (not full message content, to avoid bloating the audit log with templated text) — doc 24: who/what triggered it, recipient, channel, category, result.

## Future

Notification bundles/digests (batch low-priority notifications into a periodic summary), escalation policies (unacknowledged Critical notifications escalate to a secondary channel or on-call rotation — EE), on-call routing.
