# 50 — Frontend & Mobile-First Architecture

The console is **mobile-first**: fully usable from a phone, not a desktop UI squeezed onto a small screen. Vue 3 + PrimeVue, embedded in the single binary, permission-aware by default.

## Stack

```text
Vue 3 (Composition API + <script setup>)
Vite                build + dev server + HMR
PrimeVue            component library (DataTable, Dialog, forms, charts)
Pinia               state management
Vue Router          routing (lazy, permission-guarded)
TypeScript          end-to-end types
```

Production build (`vite build`) → `frontend/dist` → embedded via `rust-embed` into the `autotim` binary; Axum serves it with SPA fallback (doc 10).

## Mobile-First Doctrine

Design for the smallest screen first, then progressively enhance. Concretely:

1. **Single-column by default.** Layouts stack vertically on phones; multi-column appears only at `md+` breakpoints.
2. Bottom navigation on mobile, sidebar on desktop. Primary navigation sits in thumb reach on phones; it becomes a collapsible sidebar on larger screens.
3. **Touch targets ≥ 44px.** All interactive elements meet touch-size minimums; spacing tuned for fingers, not cursors.
4. **Tables become cards on mobile.** PrimeVue `DataTable` uses responsive/stacked mode below `md`: each row renders as a labeled card. Hosts, zones, jobs, audit — all readable on a phone without horizontal scrolling.
5. Cursor pagination + virtual scroll for long lists (logs, audit, events) so phones stay smooth and low-memory (pairs with doc 40).
6. Drawers and bottom sheets instead of wide modals on mobile; full dialogs on desktop.
7. Forms stack and use native inputs (correct `inputmode`/`type`) so mobile keyboards behave (numeric, email, etc.).
8. **No hover-only interactions.** Anything reachable by hover on desktop has a tap-equivalent on mobile.

## PWA — Installable & Offline-Aware

The console ships as an installable Progressive Web App:

- Web App Manifest (name, icons, theme, standalone display) → "Add to Home Screen".
- Service worker caches the app shell for fast loads and graceful offline messaging (the SPA shell loads; data requires connectivity).
- Push notifications (where supported) integrate with the Notifications module (doc 31) for alerts like *Agent Offline* / *Certificate Expiring*.
- **No service worker caching of sensitive API responses**; credentials never persisted to disk (doc 20).

This makes a phone a first-class operator device: install Autotim, get pushed alerts, act on the go.

## Responsive Breakpoints

```text
base  (< 640px)   phone       single column, bottom nav, stacked cards
sm    (≥ 640px)   large phone  slightly denser
md    (≥ 768px)   tablet       sidebar appears, 2-column where useful
lg    (≥ 1024px)  desktop      full multi-column dashboards
xl    (≥ 1280px)  wide         max content width, more density
```

PrimeVue's responsive utilities + a small set of design tokens drive these consistently.

## Layout Structure

```text
Core Layout  (provided by Core UI)
├── Top bar     (org switcher [EE], search, notifications, user menu)
├── Navigation  (bottom bar on mobile / sidebar on desktop)
└── Content     (modules render here only)
```

Modules provide content and navigation entries, never their own chrome. The shell is consistent everywhere.

## Module Frontend Registration

Mirroring the backend `Module` trait, each module ships a `FrontendManifest`:

```ts
export const dnsModule: FrontendManifest = {
  name: "dns",
  navigation: [{ title: "DNS", icon: "pi pi-globe", route: "/dns",
                 requires: "dns.zone.read", group: "Infrastructure" }],
  routes: [{ path: "/dns", component: () => import("./views/Zones.vue"),
             requires: "dns.zone.read" }],
  widgets: [{ id: "dns-health", component: () => import("./widgets/Health.vue"),
              requires: "dns.zone.read" }],
  settings: [{ title: "DNS", component: () => import("./settings/Dns.vue"),
               requires: "settings.module.update" }],
};
```

A Frontend Registry (Pinia store) collects manifests and builds navigation, routes, dashboards, and settings pages dynamically. Core never hardcodes module menus (echoes doc 13). Modules are lazy-loaded (code-split) so startup stays fast on mobile networks.

## Permission-Aware UI

The frontend mirrors RBAC for UX, **never** for security (the server is the gate — doc 21):

```vue
<Button v-if="can('dns.zone.delete', zoneScope)" @click="del" />
```

- Navigation entries, routes, widgets, buttons all declare a `requires` permission.
- The current user's resolved permissions (scoped) are loaded once and cached in Pinia, invalidated on relevant events.
- Lacking a permission hides or disables the control; the API still enforces it.

## State Management (Pinia)

- `auth` store: user, active organization, resolved permissions, session state.
- `ui` store: theme (light/dark), layout, breakpoint, mobile nav state.
- Per-module stores own module data.
- Sensitive values (tokens) live in memory; sessions ride in `HttpOnly` cookies (doc 20). **Never `localStorage` for credentials.**

## Typed API Client

The OpenAPI spec (doc 40) generates a typed TypeScript client, so the frontend is type-safe end-to-end and cannot drift from the backend contract.

## Reliability & Error Boundaries

- A failing module **must not** break the whole UI. Each module mounts behind an error boundary; a broken DNS view does not take down Mail or Settings.
- Global error/toast handling via PrimeVue `Toast`; structured API errors (doc 40) map to friendly messages.

## Accessibility

WCAG-minded: semantic markup, focus management, keyboard navigation, sufficient contrast in both themes, ARIA where PrimeVue needs reinforcement. Accessibility and mobile usability reinforce each other.

## Internationalization

`vue-i18n` from the start; all strings externalized (English source). Notification templates localize independently (doc 31).

## Performance Budget (mobile-conscious)

- Route-level code splitting; lazy module loading.
- Virtualized long lists; cursor pagination.
- Embedded assets are compressed (brotli) and cache-busted.
- Initial shell kept lean so first paint is fast on 4G.

## Observability

Track page load time, route errors, component failures (doc 51); surface frontend errors with correlation IDs back to the backend trace.

## Future

Customizable dashboards, a (sandboxed, signed) UI-extension marketplace tied to the future out-of-process plugin epic (doc 13 / 05), and QR mobile login (doc 22).

## Decision

If it does not work well on a phone, it is not done.
