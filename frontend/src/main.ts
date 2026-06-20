// Autotim Console frontend entry point.
// Mobile-first Vue 3 + PrimeVue shell (doc 50). The Frontend Registry
// (Pinia store) that collects per-module FrontendManifests and builds
// navigation/routes/widgets dynamically (doc 13, doc 50) is the next
// milestone — this scaffold wires the static shell only.

import { createApp } from "vue";
import { createPinia } from "pinia";
import PrimeVue from "primevue/config";
import App from "./App.vue";
import { router } from "./router";

const app = createApp(App);

app.use(createPinia());
app.use(router);
app.use(PrimeVue);

app.mount("#app");
