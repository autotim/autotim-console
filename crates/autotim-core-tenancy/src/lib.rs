//! Tenancy module — Core implementation.
//!
//! Owns the `organizations` and (for now) `users` tables (doc 12,
//! doc 14) and provides the `TenantContext` port implementation
//! consumed by every other module via `ModuleContext` (doc 13).
//!
//! ## Status
//!
//! `StaticTenantContext` below is deliberately the *only*
//! implementation at this stage. It is constructed directly with an
//! `OrganizationId`/`UserId`, not resolved from a session, token, or
//! HTTP request — that resolution (doc 12 §"Resolution Order for
//! organization_id") depends on Auth (doc 22) and the kernel's
//! request pipeline (doc 11), neither of which exist yet.
//!
//! This is sufficient to prove Row-Level Security end-to-end in
//! integration tests (`SET LOCAL app.org_id` + a real Postgres
//! connection), without inventing a fake session/auth layer that
//! doesn't exist. The real, request-resolved implementation is a
//! separate, later commit: `feat(kernel): connect to PostgreSQL and
//! wire TenantContext into request pipeline`.

#![forbid(unsafe_code)]

use autotim_sdk::ports::TenantContext;
use autotim_sdk::types::{OrganizationId, UserId};

/// A `TenantContext` constructed directly from a known
/// `organization_id` (and optionally a `user_id`), with no session or
/// request resolution involved.
///
/// Used today by integration tests and by any call site (jobs, CLI
/// tooling, seed scripts) that already knows which organization it is
/// acting on. Not used by the HTTP request path yet — see module docs
/// above.
#[derive(Debug, Clone, Copy)]
pub struct StaticTenantContext {
    organization_id: OrganizationId,
    user_id: Option<UserId>,
}

impl StaticTenantContext {
    pub fn new(organization_id: OrganizationId) -> Self {
        Self {
            organization_id,
            user_id: None,
        }
    }

    pub fn with_user(organization_id: OrganizationId, user_id: UserId) -> Self {
        Self {
            organization_id,
            user_id: Some(user_id),
        }
    }
}

impl TenantContext for StaticTenantContext {
    fn organization_id(&self) -> OrganizationId {
        self.organization_id
    }

    fn user_id(&self) -> Option<UserId> {
        self.user_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    // doc 14: every PK on this platform is UUID v7. Tests use
    // Uuid::now_v7() rather than new_v4() so they don't quietly rely
    // on a uuid feature ("v4") the workspace deliberately does not
    // enable (Cargo.toml only turns on "v7" and "serde").
    #[test]
    fn exposes_the_organization_it_was_constructed_with() {
        let org = OrganizationId(Uuid::now_v7());
        let ctx = StaticTenantContext::new(org);

        assert_eq!(ctx.organization_id(), org);
        assert_eq!(ctx.user_id(), None);
    }

    #[test]
    fn with_user_exposes_both_organization_and_user() {
        let org = OrganizationId(Uuid::now_v7());
        let user = UserId(Uuid::now_v7());
        let ctx = StaticTenantContext::with_user(org, user);

        assert_eq!(ctx.organization_id(), org);
        assert_eq!(ctx.user_id(), Some(user));
    }
}