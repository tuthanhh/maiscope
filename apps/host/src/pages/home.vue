<script setup lang="ts">
// Single Home screen, served at both '/' and '/:gameCode'. Hero brand +
// subtitle + two entry points: Browse and the custom-chart Visualizer.
// With one game configured the picker is unnecessary; the game is resolved
// from the route param, falling back to the first configured site.
import usePageTitle from "~/composables/usePageTitle";
import useI18n from "~/composables/useI18n";
import { GAME } from "~/app/game";
import config from "~/app/config";

defineOptions({ name: "HomePage" });

const { t } = useI18n();

usePageTitle(() => ({
    titleTemplate: "%s",
    title: `${GAME.gameTitle} | ${config.siteTitle}`,
}));
</script>

<template>
    <div class="mv-root mv-home">
        <div class="mv-home-inner">
            <h1 class="mv-home-title">
                mai<span class="mv-home-dot">·</span>scope
            </h1>
            <p class="mv-home-subtitle">maimai visualization project</p>

            <div class="mv-home-actions">
                <router-link
                    :to="{ name: 'songs' }"
                    class="mv-btn mv-home-btn mv-home-btn--primary"
                >
                    {{ t("page-title.songs") }} →
                </router-link>
                <router-link
                    :to="{ name: 'visualizer' }"
                    class="mv-btn mv-home-btn mv-home-btn--ghost"
                >
                    {{ t("page-title.visualizer") }} →
                </router-link>
            </div>
        </div>
    </div>
</template>

<style scoped>
.mv-home {
    display: flex;
    align-items: center;
    justify-content: center;
    /* fill the viewport minus header (52) + footer (~38) */
    min-height: calc(100vh - 92px);
    padding: 32px 18px;
}
.mv-home-inner {
    text-align: center;
}
.mv-home-title {
    font-size: 64px;
    font-weight: 700;
    letter-spacing: 0.04em;
    margin: 0;
    line-height: 1.05;
}
.mv-home-dot {
    color: var(--mv-mut);
}
.mv-home-subtitle {
    font-size: 18px;
    color: var(--mv-mut);
    letter-spacing: 0.04em;
    margin: 14px 0 40px;
}
.mv-home-actions {
    display: flex;
    gap: 12px;
    justify-content: center;
    flex-wrap: wrap;
}
.mv-home-btn {
    font-family: var(--mv-mono);
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0.06em;
    padding: 12px 24px;
    border-radius: 4px;
    text-decoration: none;
    transition:
        opacity 0.12s,
        border-color 0.12s;
}
.mv-home-btn--primary {
    background: var(--mv-invbg);
    color: var(--mv-inv);
    border: 1px solid var(--mv-fg);
}
.mv-home-btn--ghost {
    background: none;
    color: var(--mv-fg);
    border: 1px solid var(--mv-bd);
}
.mv-home-btn--ghost:hover {
    border-color: var(--mv-fg);
    opacity: 1;
}
</style>
