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

// Hash history: robust for a packaged Tauri app served from file://-like origins.
const router = createRouter({
  history: createWebHashHistory(),
  routes,
});

export default router;
