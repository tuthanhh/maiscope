<script setup lang="ts">
import { watch, onMounted } from "vue";
import { useDataStore } from "~/stores/data";
import useI18n from "~/composables/useI18n";
import useDarkMode from "~/composables/useDarkMode";
import usePageTitle from "~/composables/usePageTitle";
import { GAME } from "~/app/game";
import config from "~/app/config";
import MvNavigation from "~/components/layout/MvNavigation.vue";

defineOptions({ name: "DefaultLayout" });

const i18n = useI18n();
const dataStore = useDataStore();
const { isDarkMode } = useDarkMode();


usePageTitle(() => ({
    title: "N/A",
    titleTemplate: `%s | ${GAME.gameTitle} | ${config.siteTitle}`,
}));

onMounted(() => {
    dataStore.loadReference();
    dataStore.loadData();
});

// The browser picks a CJK face from the language, so <html lang> follows the
// active locale (design-direction, "Type families").
watch(
    () => i18n.localeProperties.iso ?? i18n.locale,
    (iso) => {
        document.documentElement.lang = iso;
    },
    { immediate: true },
);

watch(
    isDarkMode,
    () => {
        // Swap the semantic token layer (tokens.scss :root.dark).
        document.documentElement.classList.toggle("dark", isDarkMode.value);
    },
    { immediate: true },
);
</script>

<template>
    <div class="mv-root mv-app">
        <!-- No app-wide loading overlay: only Songs needs the Catalog, and it
             says so itself. The chart page and a song link paint from their
             own fetches and the reference tables. -->

        <MvNavigation />

        <main class="mv-main">
            <!-- No keep-alive: every route open remounts the page (and the
                 visualizer's <canvas id="bevy">), reloading fresh each time. -->
            <router-view />
        </main>

    </div>
</template>

<style scoped lang="scss">
@use "~/assets/styles/breakpoints" as bp;

.mv-app {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
}
.mv-main {
    flex: 1;
}
// Room for the fixed bottom bar on phones, while it is shown.
@media (max-width: #{bp.$bp-md - 1px}) {
    .mv-main {
        padding-bottom: calc(var(--nav-h) + env(safe-area-inset-bottom));
    }
    .mv-app:has(.mv-bottombar--hidden) .mv-main {
        padding-bottom: 0;
    }
}
</style>
