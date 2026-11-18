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
  // The song page is gone; its links open the song modal over Songs.
  {
    path: "/song",
    redirect: (to) => ({
      name: "songs",
      query: typeof to.query.id === "string" ? { song: to.query.id } : {},
    }),
  },
  {
    path: "/visualizer",
    name: "visualizer",
    component: () => import("~/pages/visualizer.vue"),
  },
  {
    path: "/community",
    name: "community",
    component: () => import("~/pages/community.vue"),
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
  // Back and Forward return to where the page was; a query change on the same
  // page (the song modal opening or switching sheet) leaves the scroll alone;
  // any other navigation starts at the top.
  scrollBehavior(to, from, savedPosition) {
    if (savedPosition) return savedPosition;
    if (to.name === from.name) return false;
    return { top: 0 };
  },
});

export default router;
