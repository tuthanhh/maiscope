// First, before the router module: see the file.
import "~/app/oauthCallback";
import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import router from "~/app/router";
import i18n from "~/app/i18n";
import { installAudioUnlock } from "~/utils/audioUnlock";
import { useSessionStore } from "~/stores/session";

import "~/assets/styles/global.scss";
import "~/assets/styles/tokens.scss";
import "~/assets/styles/theme.scss";

async function bootstrap() {
  // Must run before any audio code so it wraps AudioContext before the Bevy
  // engine's kira backend constructs one (otherwise it stays suspended forever).
  installAudioUnlock();

  const app = createApp(App);

  app.use(createPinia());
  app.use(router);
  app.use(i18n);

  // A stored session is checked in the background; the app does not wait.
  void useSessionStore().restore();

  await router.isReady();
  app.mount("#app");
}

bootstrap();
