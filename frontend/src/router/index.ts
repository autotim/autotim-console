import { createRouter, createWebHistory } from "vue-router";

// Module-contributed routes (doc 50, doc 13) are merged in dynamically
// by the Frontend Registry once modules exist. This scaffold defines
// only the platform-level routes.
export const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      name: "home",
      component: () => import("../views/Home.vue"),
    },
  ],
});
