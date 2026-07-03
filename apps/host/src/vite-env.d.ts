/// <reference types="vite/client" />

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}

declare module "*.yaml" {
  const data: Record<string, unknown>;
  export default data;
}

import type { AppConfig } from "~/app/config";

declare module "vue" {
  interface ComponentCustomProperties {
    $config: AppConfig;
    $sentry: { captureException: (err: unknown) => void };
    $gtag: (...args: unknown[]) => void;
  }
}
