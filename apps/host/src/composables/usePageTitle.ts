import { watchEffect } from 'vue';

let titleTemplate = '%s';

export default function usePageTitle(metaFn: () => { title?: string; titleTemplate?: string }) {
  watchEffect(() => {
    const meta = metaFn();
    if (typeof meta.titleTemplate === 'string') {
      titleTemplate = meta.titleTemplate;
    }
    if (typeof meta.title === 'string') {
      document.title = titleTemplate.includes('%s')
        ? titleTemplate.replace('%s', meta.title)
        : meta.title;
    }
  });
}
