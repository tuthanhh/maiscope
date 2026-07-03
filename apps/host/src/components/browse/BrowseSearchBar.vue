<script setup lang="ts">
import MvIcon from "~/components/ui/MvIcon.vue";

defineOptions({ name: "BrowseSearchBar" });

const model = defineModel<string>({ default: "" });
defineProps<{ placeholder?: string }>();
</script>

<template>
    <div class="mv-search">
        <MvIcon name="search" :size="17" />
        <input v-model="model" class="mv-search-input" :placeholder="placeholder" />
        <button
            v-if="model"
            type="button"
            class="mv-btn mv-icon-btn"
            @click="model = ''"
        >
            <MvIcon name="x" :size="16" />
        </button>
    </div>
</template>

<style scoped>
.mv-search {
    display: flex;
    align-items: center;
    gap: 11px;
    border: 1px solid var(--mv-bd);
    border-radius: 5px;
    padding: 12px 14px;
    margin-bottom: 12px;
}
.mv-search-input {
    flex: 1;
    background: none;
    border: none;
    color: var(--mv-fg);
    font-family: var(--mv-mono);
    font-size: 14px;
}
.mv-icon-btn {
    background: none;
    border: none;
    padding: 0;
    display: flex;
    color: var(--mv-mut);
}
</style>
