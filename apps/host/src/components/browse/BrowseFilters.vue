<script setup lang="ts">
import { ref } from "vue";
import useI18n from "~/composables/useI18n";
import useGameData from "~/composables/useGameData";
import MvIcon from "~/components/ui/MvIcon.vue";
import MvChip from "~/components/ui/MvChip.vue";
import MvField from "~/components/ui/MvField.vue";
import MvTextInput from "~/components/ui/MvTextInput.vue";
import MvSelect, { type SelectOption } from "~/components/ui/MvSelect.vue";

export type BrowseForm = {
    title: string;
    artist: string;
    designer: string;
    category: string;
    version: string;
    types: string[];
    difficulties: string[];
    levelMin: string;
    levelMax: string;
    bpmMin: string;
    bpmMax: string;
};

defineOptions({ name: "BrowseFilters" });

const props = defineProps<{
    form: BrowseForm;
    categoryOptions: SelectOption[];
    versionOptions: SelectOption[];
    designerOptions: SelectOption[];
    levelOptions: SelectOption[];
    typeChips: { type: string }[];
    difficultyChips: { difficulty: string; name?: string }[];
}>();

const emit = defineEmits<{ draw: []; reset: [] }>();

const { t } = useI18n();
const { getTypeAbbr } = useGameData();

const advOpen = ref(true);

// Toggle a value in/out of a multi-select array (chip groups).
function toggle(list: string[], value: string): string[] {
    return list.includes(value)
        ? list.filter((v) => v !== value)
        : [...list, value];
}
</script>

<template>
    <div class="mv-adv">
        <button
            type="button"
            class="mv-btn mv-adv-head"
            @click="advOpen = !advOpen"
        >
            <span class="mv-adv-title">
                <MvIcon name="sliders" :size="15" />
                <span>{{ t("page.songs.advancedSearch") }}</span>
            </span>
            <span class="mv-adv-chevron" :class="{ closed: !advOpen }">
                <MvIcon name="chevron" :size="13" />
            </span>
        </button>

        <div v-if="advOpen" class="mv-adv-body">
            <div class="mv-fields">
                <MvField :label="t('term.artist')">
                    <MvTextInput
                        v-model="props.form.artist"
                        placeholder="—"
                        :width="150"
                    />
                </MvField>
                <MvField :label="t('term.noteDesigner')">
                    <MvSelect
                        v-model="props.form.designer"
                        :options="designerOptions"
                        :placeholder="t('ui.all')"
                        :width="160"
                    />
                </MvField>
                <MvField :label="t('term.category')">
                    <MvSelect
                        v-model="props.form.category"
                        :options="categoryOptions"
                        :placeholder="t('ui.all')"
                        :width="160"
                    />
                </MvField>
                <MvField :label="t('term.version')">
                    <MvSelect
                        v-model="props.form.version"
                        :options="versionOptions"
                        :placeholder="t('ui.all')"
                        :width="160"
                    />
                </MvField>
            </div>

            <div class="mv-fields mv-fields--wide">
                <MvField :label="t('term.type')">
                    <div class="mv-chip-row">
                        <MvChip
                            v-for="ty in typeChips"
                            :key="ty.type"
                            :active="props.form.types.includes(ty.type)"
                            @click="
                                props.form.types = toggle(
                                    props.form.types,
                                    ty.type,
                                )
                            "
                        >
                            {{ getTypeAbbr(ty.type) }}
                        </MvChip>
                    </div>
                </MvField>
                <MvField :label="t('term.difficulty')">
                    <div class="mv-chip-row">
                        <MvChip
                            v-for="d in difficultyChips"
                            :key="d.difficulty"
                            :active="
                                props.form.difficulties.includes(d.difficulty)
                            "
                            @click="
                                props.form.difficulties = toggle(
                                    props.form.difficulties,
                                    d.difficulty,
                                )
                            "
                        >
                            {{ d.name }}
                        </MvChip>
                    </div>
                </MvField>
                <MvField :label="t('term.level')">
                    <div class="mv-range">
                        <MvSelect
                            v-model="props.form.levelMin"
                            :options="levelOptions"
                            :placeholder="t('page.songs.min')"
                            :width="90"
                        />
                        <span class="mv-dash">–</span>
                        <MvSelect
                            v-model="props.form.levelMax"
                            :options="levelOptions"
                            :placeholder="t('page.songs.max')"
                            :width="90"
                        />
                    </div>
                </MvField>
                <MvField :label="t('term.bpm')">
                    <div class="mv-range">
                        <MvTextInput
                            v-model="props.form.bpmMin"
                            :placeholder="t('page.songs.min')"
                            :width="80"
                        />
                        <span class="mv-dash">–</span>
                        <MvTextInput
                            v-model="props.form.bpmMax"
                            :placeholder="t('page.songs.max')"
                            :width="80"
                        />
                    </div>
                </MvField>
            </div>

            <div class="mv-adv-actions">
                <button
                    type="button"
                    class="mv-btn mv-action-primary"
                    @click="emit('draw')"
                >
                    ⤮ {{ t("page.songs.drawRandom") }}
                </button>
                <button
                    type="button"
                    class="mv-btn mv-action-ghost"
                    @click="emit('reset')"
                >
                    {{ t("page.songs.reset") }}
                </button>
            </div>
        </div>
    </div>
</template>

<style scoped>
.mv-adv {
    border: 1px solid var(--mv-bd);
    border-radius: 5px;
    margin-bottom: 18px;
}
.mv-adv-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    background: none;
    border: none;
    color: var(--mv-fg);
    padding: 12px 14px;
}
.mv-adv-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
}
.mv-adv-chevron {
    color: var(--mv-mut);
    transition: transform 0.15s;
}
.mv-adv-chevron.closed {
    transform: rotate(-90deg);
}
.mv-adv-body {
    padding: 4px 14px 16px;
    border-top: 1px solid var(--mv-bd2);
}
.mv-fields {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    margin-top: 14px;
}
.mv-fields--wide {
    gap: 24px;
    align-items: flex-end;
}
.mv-chip-row {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
}
.mv-range {
    display: flex;
    align-items: center;
    gap: 6px;
}
.mv-dash {
    color: var(--mv-mut);
}
.mv-adv-actions {
    display: flex;
    gap: 8px;
    margin-top: 18px;
}
.mv-action-primary {
    background: var(--mv-invbg);
    color: var(--mv-inv);
    border: none;
    border-radius: 3px;
    font-size: 11px;
    font-weight: 700;
    padding: 8px 16px;
    letter-spacing: 0.08em;
}
.mv-action-ghost {
    background: none;
    color: var(--mv-mut);
    border: 1px solid var(--mv-bd);
    border-radius: 3px;
    font-size: 11px;
    padding: 8px 16px;
}
</style>
