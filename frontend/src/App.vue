<script setup lang="ts">
// Core Layout shell (doc 50): top bar + navigation (bottom on mobile,
// sidebar on desktop, via CSS below) + content. Modules render only
// into <router-view/>; they never provide their own chrome.
</script>

<template>
  <div class="app-shell">
    <header class="topbar">
      <span class="brand">Autotim Console</span>
    </header>

    <main class="content">
      <router-view />
    </main>

    <nav class="bottom-nav">
      <!-- Populated dynamically by the Frontend Registry once modules
           register navigation entries (doc 13, doc 50). -->
    </nav>
  </div>
</template>

<style>
/* Mobile-first base layout (doc 50). Single column by default; the
   sidebar/bottom-nav split happens at the md breakpoint once real
   navigation content exists. */
.app-shell {
  display: flex;
  flex-direction: column;
  min-height: 100vh;
}
.topbar {
  padding: 0.75rem 1rem;
  border-bottom: 1px solid #e2e8f0;
}
.content {
  flex: 1;
  padding: 1rem;
}
.bottom-nav {
  border-top: 1px solid #e2e8f0;
  min-height: 56px; /* touch-target friendly, doc 50 */
}
</style>
