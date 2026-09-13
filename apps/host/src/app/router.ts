// Manual route table replacing Nuxt's file-based routing under pages/.
// Single-game (maimai) app: no /:gameCode param.
import {
  createRouter,
  createWebHashHistory,
  type RouteRecordRaw,
} from "vue-router";

const routes: RouteRecordRaw[] = [
  {
    path: "/",
    name: "home",
    component: () => import("~/pages/home.vue"),
  },
  {
    path: "/songs",
    name: "songs",
    component: () => import("~/pages/songs.vue"),
  },
  {
    path: "/song",
    name: "song",
    component: () => import("~/pages/song.vue"),
  },
  {
    path: "/visualizer",
    name: "visualizer",
    component: () => import("~/pages/visualizer.vue"),
  },
  {
    path: "/about",
    name: "about",
    component: () => import("~/pages/about.vue"),
  },
];

// Hash history: the route lives in the URL fragment, which the server never sees,
// so deep links resolve on Cloudflare Pages without needing an SPA fallback/rewrite rule.
const router = createRouter({
  history: createWebHashHistory(),
  routes,
});

export default router;
