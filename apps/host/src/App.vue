<script setup lang="ts">
import { computed, watch, onMounted } from "vue";
import { useDataStore } from "~/stores/data";
import useI18n from "~/composables/useI18n";
import useDarkMode from "~/composables/useDarkMode";
import usePageTitle from "~/composables/usePageTitle";
import LoadingStatus from "~/enums/LoadingStatus";
import { GAME } from "~/app/game";
import config from "~/app/config";
import MvIcon from "~/components/ui/MvIcon.vue";
import MvLocaleMenu from "~/components/ui/MvLocaleMenu.vue";
import MvLoadingOverlay from "~/components/ui/MvLoadingOverlay.vue";
import MvSheetDialog from "~/components/dialogs/MvSheetDialog.vue";

defineOptions({ name: "DefaultLayout" });

const { t } = useI18n();
const dataStore = useDataStore();
const { isDarkMode } = useDarkMode();

const navLinks = computed(() => [
    { label: t("page-title.home"), to: { name: "home" } },
    { label: t("page-title.songs"), to: { name: "songs" } },
    { label: t("page-title.visualizer"), to: { name: "visualizer" } },
    { label: t("page-title.about"), to: { name: "about" } },
]);
const externalLinks = computed(() => [
    {
        label: t("page-title.bug-report"),
        href: config.siteReportUrl,
        icon: "ext" as const,
    },
    {
        label: t("page-title.source-code"),
        href: config.sourceCodeUrl,
        icon: "ext" as const,
    },
]);

usePageTitle(() => ({
    title: "N/A",
    titleTemplate: `%s | ${GAME.gameTitle} | ${config.siteTitle}`,
}));

onMounted(() => {
    dataStore.loadData();
});

watch(
    isDarkMode,
    () => {
        // Flip the monochrome design tokens (theme.scss :root.dark).
        document.documentElement.classList.toggle("dark", isDarkMode.value);
    },
    { immediate: true },
);
</script>

<template>
    <div class="mv-root mv-app">
        <MvLoadingOverlay
            v-if="dataStore.currentLoadingStatus === LoadingStatus.LOADING"
        />

        <header class="mv-header">
            <div class="mv-header-left">
                <router-link to="/" class="mv-brand">
                    mai<span class="mv-brand-dot">·</span>scope
                </router-link>
                <nav v-if="navLinks.length > 0" class="mv-nav">
                    <router-link
                        v-for="link in navLinks"
                        :key="link.label"
                        :to="link.to"
                        class="mv-nav-link"
                    >
                        {{ link.label }}
                    </router-link>
                </nav>
            </div>

            <div class="mv-header-right">
                <a
                    v-for="ext in externalLinks"
                    :key="ext.label"
                    :href="ext.href"
                    target="_blank"
                    rel="noopener"
                    class="mv-btn mv-ext"
                    :title="ext.label"
                >
                    <MvIcon :name="ext.icon" :size="15" />
                </a>
                <MvLocaleMenu />
                <button
                    type="button"
                    class="mv-btn mv-toggle"
                    :title="isDarkMode ? 'Light' : 'Dark'"
                    @click="isDarkMode = !isDarkMode"
                >
                    <MvIcon :name="isDarkMode ? 'sun' : 'moon'" :size="16" />
                </button>
            </div>
        </header>

        <main class="mv-main">
            <!-- No keep-alive: every route open remounts the page (and the
                 visualizer's <canvas id="bevy">), reloading fresh each time. -->
            <router-view />
        </main>

        <footer class="mv-footer">
            <span>{{ config.siteTitle }}</span>
            <span>made by tuthanhh and claude</span>
        </footer>

        <MvSheetDialog />
    </div>
</template>

<style scoped>
.mv-app {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
}

.mv-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 18px;
    height: 52px;
    border-bottom: 1px solid var(--mv-bd);
}
.mv-header-left {
    display: flex;
    align-items: center;
    gap: 22px;
}
.mv-brand {
    font-weight: 700;
    letter-spacing: 0.04em;
    font-size: 15px;
    color: var(--mv-fg);
    text-decoration: none;
}
.mv-brand-dot {
    color: var(--mv-mut);
}
.mv-nav {
    display: flex;
    gap: 4px;
}
.mv-nav-link {
    color: var(--mv-mut);
    font-size: 12px;
    padding: 6px 8px;
    text-decoration: none;
    border-bottom: 1px solid transparent;
}
.mv-nav-link.router-link-exact-active {
    color: var(--mv-fg);
    border-bottom-color: var(--mv-fg);
}

.mv-header-right {
    display: flex;
    align-items: center;
    gap: 6px;
}
.mv-ext,
.mv-toggle {
    display: flex;
    background: none;
    border: 1px solid var(--mv-bd);
    border-radius: 3px;
    padding: 6px;
    color: var(--mv-fg);
}

.mv-main {
    flex: 1;
}

.mv-footer {
    border-top: 1px solid var(--mv-bd);
    padding: 10px 18px;
    font-size: 10px;
    color: var(--mv-mut);
    display: flex;
    justify-content: space-between;
}
</style>
