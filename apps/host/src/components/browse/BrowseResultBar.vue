<script setup lang="ts">
import useI18n from "~/composables/useI18n";
import MvIcon from "~/components/ui/MvIcon.vue";

defineOptions({ name: "BrowseResultBar" });

defineProps<{ count: number; bookmarkCount: number }>();
const gridView = defineModel<boolean>("gridView", { default: false });
const myListOnly = defineModel<boolean>("myListOnly", { default: false });

const { t } = useI18n();
</script>

<template>
    <div class="mv-result-bar">
        <span class="mv-label">{{ count }} {{ t("term.sheets") }}</span>
        <div class="mv-result-tools">
            <button
                type="button"
                class="mv-btn mv-mylist"
                :class="{ active: myListOnly }"
                @click="myListOnly = !myListOnly"
            >
                <MvIcon name="bookmark" :size="14" :filled="myListOnly" />
                {{ t("page.songs.myList")
                }}{{ bookmarkCount > 0 ? ` · ${bookmarkCount}` : "" }}
            </button>
            <div class="mv-divider" />
            <button
                type="button"
                class="mv-btn mv-view-btn"
                :class="{ active: !gridView }"
                @click="gridView = false"
            >
                <MvIcon name="list" :size="15" />
            </button>
            <button
                type="button"
                class="mv-btn mv-view-btn"
                :class="{ active: gridView }"
                @click="gridView = true"
            >
                <MvIcon name="grid" :size="15" />
            </button>
        </div>
    </div>
</template>

<style scoped>
.mv-result-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 10px;
}
.mv-result-tools {
    display: flex;
    gap: 6px;
    align-items: center;
}
.mv-mylist {
    display: flex;
    align-items: center;
    gap: 6px;
    background: none;
    color: var(--mv-mut);
    border: 1px solid var(--mv-bd);
    border-radius: 3px;
    padding: 5px 10px;
    font-size: 11px;
}
.mv-mylist.active {
    background: var(--mv-invbg);
    color: var(--mv-inv);
    border-color: var(--mv-fg);
}
.mv-divider {
    width: 1px;
    height: 16px;
    background: var(--mv-bd);
    margin: 0 2px;
}
.mv-view-btn {
    background: none;
    border: 1px solid var(--mv-bd);
    border-radius: 3px;
    padding: 5px;
    display: flex;
    color: var(--mv-mut);
}
.mv-view-btn.active {
    background: var(--mv-panel2);
    color: var(--mv-fg);
}
</style>
