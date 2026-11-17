<script setup lang="ts">
import useI18n from "~/composables/useI18n";

// Manual paste: the fallback for a sheet with no chart, and the way to play
// any simai text. The text is two-way: a deep-linked chart fills it too.
defineProps<{ loading: boolean; engineReady: boolean; errorMsg: string }>();
defineEmits<{ load: [] }>();
const simai = defineModel<string>({ required: true });

const { t } = useI18n();
</script>

<template>
    <div class="mv-paste">
        <div class="mv-label mv-side-label">
            {{ t("page.viz.pasteTitle") }}
        </div>
        <textarea
            v-model="simai"
            class="mv-textarea"
            rows="12"
            :placeholder="t('page.viz.pastePlaceholder')"
        />

        <div class="mv-paste-row">
            <span class="mv-paste-hint">
                {{ t("page.viz.pasteHint") }}
            </span>
            <button
                type="button"
                class="mv-btn mv-parse"
                :disabled="loading || !simai.trim() || !engineReady"
                @click="$emit('load')"
            >
                {{
                    loading ? t("page.viz.loading") : t("page.viz.load")
                }}
            </button>
        </div>
        <div v-if="errorMsg && engineReady" class="mv-load-err">
            {{ errorMsg }}
        </div>
    </div>
</template>

<style scoped>
.mv-side-label {
    margin-bottom: 6px;
}
.mv-paste {
    margin-top: 22px;
    border-top: 1px solid var(--mv-bd);
    padding-top: 16px;
}
.mv-textarea {
    width: 100%;
    box-sizing: border-box;
    min-height: 240px;
    background: var(--mv-panel);
    border: 1px solid var(--mv-bd);
    border-radius: 4px;
    color: var(--mv-fg);
    font-family: var(--mv-mono);
    font-size: 13px;
    line-height: 1.5;
    padding: 12px;
    resize: vertical;
}
.mv-paste-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 12px;
}
.mv-paste-hint {
    font-size: 10px;
    color: var(--mv-mut);
}
.mv-parse {
    background: var(--mv-invbg);
    color: var(--mv-inv);
    border: none;
    border-radius: 3px;
    font-family: var(--mv-mono);
    font-size: 11px;
    padding: 6px 14px;
    font-weight: 700;
}
.mv-parse:disabled {
    opacity: 0.45;
    cursor: not-allowed;
}
.mv-load-err {
    margin-top: 10px;
    font-size: 10px;
    color: #e66;
    font-family: var(--mv-mono);
}
</style>
