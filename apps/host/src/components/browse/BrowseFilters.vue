<script setup lang="ts">
import useI18n from "~/composables/useI18n";
import useGameData from "~/composables/useGameData";
import MvChip from "~/components/ui/MvChip.vue";
import MvField from "~/components/ui/MvField.vue";
import MvTextInput from "~/components/ui/MvTextInput.vue";
import MvCombobox from "~/components/ui/MvCombobox.vue";
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
    region: string;
    useRegionOverride: boolean;
    matchExactTitle: boolean;
    matchExactArtist: boolean;
    useInternalLevel: boolean;
};

// The drawer's fields: everything the browse surface does not carry
// (design-direction "Browse"). Toggles are filter chips. Each control writes
// the form directly, so filtering is live.
defineOptions({ name: "BrowseFilters" });

const props = defineProps<{
    form: BrowseForm;
    categoryOptions: SelectOption[];
    versionOptions: SelectOption[];
    designerOptions: SelectOption[];
    regionOptions: SelectOption[];
    /** Every artist in the Catalog, sorted, for the artist picker. */
    artistOptions: string[];
    typeChips: { type: string }[];
}>();

const { t } = useI18n();
const { getTypeAbbr } = useGameData();

function toggleType(type: string): void {
    const types = props.form.types;
    props.form.types = types.includes(type)
        ? types.filter((v) => v !== type)
        : [...types, type];
}
</script>

<template>
    <MvField :label="t('term.title')" group>
        <span class="mv-filter-chips">
            <MvChip
                :active="form.matchExactTitle"
                @click="form.matchExactTitle = !form.matchExactTitle"
            >
                {{ t("page.songs.exactMatch") }}
            </MvChip>
        </span>
    </MvField>
    <MvField :label="t('term.artist')">
        <MvCombobox
            v-model="form.artist"
            class="mv-filter-control"
            :options="artistOptions"
            :placeholder="t('ui.all') as string"
            :more-label="t('page.songs.typeToNarrow') as string"
        />
        <span class="mv-filter-chips">
            <MvChip
                :active="form.matchExactArtist"
                @click="form.matchExactArtist = !form.matchExactArtist"
            >
                {{ t("page.songs.exactMatch") }}
            </MvChip>
        </span>
    </MvField>
    <MvField :label="t('term.noteDesigner')">
        <MvSelect
            v-model="form.designer"
            class="mv-filter-control"
            :options="designerOptions"
            :placeholder="t('ui.all')"
        />
    </MvField>
    <MvField :label="t('term.category')">
        <MvSelect
            v-model="form.category"
            class="mv-filter-control"
            :options="categoryOptions"
            :placeholder="t('ui.all')"
        />
    </MvField>
    <MvField :label="t('term.version')">
        <MvSelect
            v-model="form.version"
            class="mv-filter-control"
            :options="versionOptions"
            :placeholder="t('ui.all')"
        />
    </MvField>
    <MvField :label="t('term.region')">
        <MvSelect
            v-model="form.region"
            class="mv-filter-control"
            :options="regionOptions"
            :placeholder="t('ui.all')"
        />
        <span class="mv-filter-chips">
            <MvChip
                :active="form.useRegionOverride"
                @click="form.useRegionOverride = !form.useRegionOverride"
            >
                {{ t("page.songs.useRegionOverride") }}
            </MvChip>
        </span>
    </MvField>
    <MvField :label="t('term.type')" group>
        <span class="mv-filter-chips">
            <MvChip
                v-for="ty in typeChips"
                :key="ty.type"
                :active="form.types.includes(ty.type)"
                @click="toggleType(ty.type)"
            >
                {{ getTypeAbbr(ty.type) }}
            </MvChip>
        </span>
    </MvField>
    <MvField :label="t('term.level')" group>
        <span class="mv-filter-chips">
            <MvChip
                :active="form.useInternalLevel"
                @click="form.useInternalLevel = !form.useInternalLevel"
            >
                {{ t("page.songs.useInternalLevel") }}
            </MvChip>
        </span>
    </MvField>
    <MvField :label="t('term.bpm')" group>
        <span class="mv-filter-range">
            <MvTextInput
                v-model="form.bpmMin"
                :placeholder="t('page.songs.min')"
                :label="t('page.songs.min')"
                inputmode="decimal"
                :width="96"
            />
            <span class="t-muted" aria-hidden="true">–</span>
            <MvTextInput
                v-model="form.bpmMax"
                :placeholder="t('page.songs.max')"
                :label="t('page.songs.max')"
                inputmode="decimal"
                :width="96"
            />
        </span>
    </MvField>
</template>

<style scoped>
.mv-filter-control {
    width: 100%;
}
.mv-filter-chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s-2);
}
.mv-filter-range {
    display: flex;
    align-items: center;
    gap: var(--s-2);
}
</style>
