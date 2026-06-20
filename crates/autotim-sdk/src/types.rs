//! Shared types referenced across every module and Core port.
//!
//! See architecture doc 12 (Tenancy) and doc 21 (RBAC).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A tenant boundary. Every tenant-scoped row, request, event, and job
/// carries one of these. See doc 12 — Tenancy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrganizationId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub Uuid);

/// Canonical permission identifier, format `module.resource.action`
/// (lowercase, dot-separated). Declared as typed constants by modules,
/// never as free-form strings at call sites. Enforced by the CI
/// permission linter. See doc 21 — RBAC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Permission(pub &'static str);

impl Permission {
    pub const fn new(s: &'static str) -> Self {
        Permission(s)
    }
}

/// Where a grant or an authorization check applies. Always anchored to
/// an organization — there is no scope without a tenant. See doc 12
/// (Tenancy) and doc 21 (RBAC) for the full model and rationale (this
/// replaces the v1 anti-pattern of encoding resource IDs into
/// permission strings).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scope {
    Organization(OrganizationId),
    Group {
        org: OrganizationId,
        group_id: Uuid,
    },
    ResourceType {
        org: OrganizationId,
        kind: &'static str,
    },
    Resource {
        org: OrganizationId,
        kind: &'static str,
        id: Uuid,
    },
}

impl Scope {
    pub fn organization(&self) -> OrganizationId {
        match self {
            Scope::Organization(org)
            | Scope::Group { org, .. }
            | Scope::ResourceType { org, .. }
            | Scope::Resource { org, .. } => *org,
        }
    }
}
