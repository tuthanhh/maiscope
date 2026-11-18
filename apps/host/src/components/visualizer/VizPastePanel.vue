<script setup lang="ts">
import useI18n from "~/composables/useI18n";
import MvButton from "~/components/ui/MvButton.vue";
import MvField from "~/components/ui/MvField.vue";
import MvTextarea from "~/components/ui/MvTextarea.vue";

// The chart text panel (design-direction "Chart page"): the textarea and a
// primary Load. Manual paste is the fallback for a sheet with no chart and
// the way to play any simai text; a deep-linked chart fills it too.
defineProps<{ loading: boolean; engineReady: boolean; errorMsg: string }>();
defineEmits<{ load: [] }>();
const simai = defineModel<string>({ required: true });

const { t } = useI18n();
</script>

<template>
    <section class="viz-paste">
        <MvField
            class="viz-paste-field"
            :label="t('page.viz.chartText') as string"
            :help="t('page.viz.pasteHint') as string"
            :error="engineReady && errorMsg ? errorMsg : undefined"
        >
            <MvTextarea
                v-model="simai"
                class="viz-paste-text"
                mono
                :rows="14"
                :placeholder="t('page.viz.pastePlaceholder') as string"
            />
        </MvField>
        <MvButton
            kind="primary"
            :disabled="loading || !simai.trim() || !engineReady"
            @click="$emit('load')"
        >
            {{ loading ? t("page.viz.loading") : t("page.viz.load") }}
        </MvButton>
    </section>
</template>

<style scoped>
.viz-paste {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--s-3);
    height: 100%;
}
.viz-paste-field {
    flex: 1;
    width: 100%;
    margin-bottom: 0;
}
.viz-paste-text {
    flex: 1;
    width: 100%;
    min-height: 240px;
    resize: vertical;
}
</style>
