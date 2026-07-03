import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import router from "~/app/router";
import i18n from "~/app/i18n";
import config from "~/app/config";
import { installAudioUnlock } from "~/utils/audioUnlock";

import "~/assets/styles/global.scss";
import "~/assets/styles/theme.scss";

async function bootstrap() {
  // Must run before any audio code so it wraps AudioContext before the Bevy
  // engine's kira backend constructs one (otherwise it stays suspended forever).
  installAudioUnlock();

  const app = createApp(App);

  app.use(createPinia());
  app.use(router);
  app.use(i18n);

  app.config.globalProperties.$config = config;
  app.config.globalProperties.$gtag = (..._args: unknown[]) => {};

  await router.isReady();
  app.mount("#app");
}

bootstrap();
