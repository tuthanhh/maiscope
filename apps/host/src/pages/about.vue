<script setup lang="ts">
import usePageTitle from "~/composables/usePageTitle";
import useI18n from "~/composables/useI18n";

defineOptions({ name: "GameAboutPage" });

const { t } = useI18n();

const roadmap = [
    {
        id: "C1",
        text: "Point client at backend read API, replacing CloudFront; keep current behavior",
    },
    {
        id: "C2",
        text: "Client-side catalog cache (ETag, then IndexedDB) + offline-first + delta sync via manifest",
    },
    {
        id: "C3",
        text: "GitHub login + contribution UI (submit chart, my contributions)",
    },
    { id: "C4", text: "Chart / audio download + maimai visualization engine" },
];

usePageTitle(() => ({ title: t("page-title.about") as string }));
</script>

<template>
    <!-- eslint-disable max-len -->
    <div class="mv-root mv-about">
        <h1 class="mv-about-h1">{{ t("page-title.about") }}</h1>
        <div class="mv-hr" />

        <h3 class="mv-about-h3">About The Project</h3>
        <p class="mv-about-p">
            <strong>maiscope</strong> is a desktop song &amp; chart browser for
            arcade rhythm games such as
            <a href="https://maimai.sega.jp/" target="_blank" rel="noopener"
                >maimai</a
            >. It loads community-maintained song data and lets you browse,
            filter, and inspect charts across difficulties.
        </p>
        <p class="mv-about-p">
            It is a desktop port of
            <a
                href="https://github.com/zetaraku/arcade-songs"
                target="_blank"
                rel="noopener"
                >zetaraku/arcade-songs</a
            >
            — the original web app — wrapped in
            <a href="https://tauri.app/" target="_blank" rel="noopener"
                >Tauri</a
            >
            for a native window and bundled distribution.
        </p>

        <h3 class="mv-about-h3">{{ t("page.about.author") }}</h3>
        <ul class="mv-about-list">
            <li>
                <a
                    href="https://github.com/tuthanhh"
                    target="_blank"
                    rel="noopener"
                    >tuthanhh</a
                >
                with the help of
                <a href="https://claude.com/" target="_blank" rel="noopener"
                    >Claude</a
                >
            </li>
        </ul>

        <h3 class="mv-about-h3">{{ t("page.about.openSource") }}</h3>
        <ul class="mv-about-list">
            <li>
                <a
                    href="https://github.com/tuthanhh/maiscope-frontend"
                    target="_blank"
                    rel="noopener"
                    >tuthanhh/maiscope-frontend</a
                >
                － this project.
                <a
                    href="https://github.com/tuthanhh/maiscope-frontend/issues/new?labels=bug"
                    target="_blank"
                    rel="noopener"
                    >Report a bug</a
                >
                ·
                <a
                    href="https://github.com/tuthanhh/maiscope-frontend/issues/new?labels=enhancement"
                    target="_blank"
                    rel="noopener"
                    >Request a feature</a
                >
            </li>
        </ul>

        <h3 class="mv-about-h3">Roadmap</h3>
        <ul class="mv-about-list">
            <li v-for="item in roadmap" :key="item.id">
                <strong>{{ item.id }}</strong> － {{ item.text }}
            </li>
        </ul>

        <h3 class="mv-about-h3">Acknowledgments</h3>
        <ul class="mv-about-list">
            <li>
                <a
                    href="https://github.com/zetaraku/arcade-songs"
                    target="_blank"
                    rel="noopener"
                    >zetaraku/arcade-songs</a
                >
                by
                <a
                    href="https://github.com/zetaraku"
                    target="_blank"
                    rel="noopener"
                    >zetaraku (Raku Zeta)</a
                >
                － the original web app this project is built on. All credit for
                the data model, filtering logic, and UI design goes to the
                upstream project.
            </li>
            <li>
                <a href="https://maimai.sega.jp/" target="_blank" rel="noopener"
                    >maimai でらっくす 公式サイト｜セガ</a
                >
                － official song information.
            </li>
            <li>The maimai fan community － chart data and metadata.</li>
        </ul>

        <h3 class="mv-about-h3">License</h3>
        <p class="mv-about-p">Distributed under the MIT License.</p>

        <h3 class="mv-about-h3">{{ t("page.about.disclaimer") }}</h3>
        <ul class="mv-about-list">
            <li>
                <strong>maimai でらっくす</strong> and
                <strong>maimai DX</strong> are trademarks of
                <strong>SEGA Interactive Co., Ltd.</strong>
            </li>
            <li>
                This site is not associated or officially connected with
                <strong>maimai でらっくす</strong> or
                <strong>SEGA Interactive Co., Ltd.</strong>
            </li>
        </ul>
    </div>
</template>

<style scoped>
.mv-about {
    padding: 18px;
    max-width: 820px;
    font-size: 13px;
    line-height: 1.7;
}
.mv-about-h1 {
    font-size: 24px;
    font-weight: 700;
}
.mv-hr {
    height: 1px;
    background: var(--mv-bd);
    margin: 12px 0 20px;
}
.mv-about-h3 {
    font-size: 12px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--mv-mut);
    margin: 24px 0 8px;
}
.mv-about-p {
    margin: 0 0 10px;
}
.mv-about-list {
    padding-left: 18px;
}
.mv-about-list li {
    margin-bottom: 6px;
}
.mv-about a {
    color: var(--mv-fg);
    text-underline-offset: 2px;
}
.mv-about a:hover {
    opacity: 0.7;
}
</style>
