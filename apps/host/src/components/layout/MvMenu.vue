<script setup lang="ts">
import { computed, onBeforeUnmount, ref, useId, watch } from "vue";
import { useRoute } from "vue-router";
import useI18n from "~/composables/useI18n";
import config from "~/app/config";
import type { LocaleObject } from "~/app/i18n";
import MvButton from "~/components/ui/MvButton.vue";
import MvSelect from "~/components/ui/MvSelect.vue";
import MvModal from "~/components/ui/MvModal.vue";
import MvError from "~/components/ui/MvError.vue";
import { useSessionStore } from "~/stores/session";

// The account menu: the account (sign in, or who is signed in, sign out and
// account deletion — frontend-redesign 20), Locale, About and the external
// links. Signed in, the avatar is the trigger. A disclosure, not an ARIA
// menu: its contents are buttons, a select and links, reached with Tab.
const i18n = useI18n();
const { t } = i18n;
const panelId = useId();
const route = useRoute();
const session = useSessionStore();

function signIn(): void {
  session.signIn(route.fullPath);
}
function signOut(): void {
  open.value = false;
  session.signOut();
}

// Deleting asks first, in a dialog that says what it does (contract §4:
// identity removal, published work kept).
const confirmTitleId = useId();
const confirming = ref(false);
const deleting = ref(false);
const deleteFailed = ref(false);
function askDelete(): void {
  open.value = false;
  deleteFailed.value = false;
  confirming.value = true;
}
async function confirmDelete(): Promise<void> {
  deleting.value = true;
  deleteFailed.value = false;
  try {
    await session.deleteAccount();
    closeConfirm();
  } catch {
    // A refused session has already signed out: nothing left to delete here.
    if (!session.signedIn) closeConfirm();
    else deleteFailed.value = true;
  } finally {
    deleting.value = false;
  }
}
// The trigger that opened the menu held focus; the panel closed under it, so
// the dialog has nothing to return to. Give focus back to the trigger.
function closeConfirm(): void {
  confirming.value = false;
  requestAnimationFrame(() =>
    root.value?.querySelector<HTMLElement>(".mv-menu-trigger")?.focus(),
  );
}
const root = ref<HTMLElement | null>(null);
const open = ref(false);

const locale = computed({
  get: () => i18n.localeProperties.code,
  set: (code: string) => i18n.setLocale(code),
});
const localeOptions = computed(() =>
  (i18n.locales as LocaleObject[]).map((l) => ({ value: l.code, text: l.name ?? l.code })),
);

// Close on a press outside. `pointerdown` rather than `focusout`: a click on
// the panel's padding, or a Safari click that never moves focus, would
// otherwise close it wrongly or not at all.
function onPointerDown(event: PointerEvent): void {
  if (!root.value?.contains(event.target as Node)) open.value = false;
}
// Escape is heard on the document: a click on the panel's padding leaves
// focus on <body>, outside this element.
function onKeyDown(event: KeyboardEvent): void {
  if (event.key !== "Escape") return;
  open.value = false;
  root.value?.querySelector<HTMLElement>(".mv-menu-trigger")?.focus();
}
function listen(on: boolean): void {
  if (on) {
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
  } else {
    document.removeEventListener("pointerdown", onPointerDown);
    document.removeEventListener("keydown", onKeyDown);
  }
}
watch(open, listen);
onBeforeUnmount(() => listen(false));

// Close when focus leaves by keyboard (Tab past the last link).
function onFocusOut(event: FocusEvent): void {
  const next = event.relatedTarget as Node | null;
  if (next && !root.value?.contains(next)) open.value = false;
}
</script>

<template>
  <div ref="root" class="mv-menu" @focusout="onFocusOut">
    <button
      v-if="session.user"
      type="button"
      class="mv-menu-trigger mv-menu-avatar"
      :aria-label="t('auth.accountMenu', { login: session.user.login }) as string"
      :title="session.user.login"
      :aria-expanded="open"
      :aria-controls="panelId"
      @click="open = !open"
    >
      <img
        v-if="session.user.avatarUrl"
        :src="session.user.avatarUrl"
        alt=""
        width="28"
        height="28"
        referrerpolicy="no-referrer"
      />
      <span v-else class="t-strong">{{ session.user.login.slice(0, 1).toUpperCase() }}</span>
    </button>
    <MvButton
      v-else
      kind="icon"
      icon="user"
      class="mv-menu-trigger"
      :label="t('nav.menu') as string"
      :aria-expanded="open"
      :aria-controls="panelId"
      @click="open = !open"
    />
    <div v-if="open" :id="panelId" class="mv-menu-panel">
      <div v-if="session.user" class="mv-menu-account">
        <span class="t-strong">{{ session.user.login }}</span>
        <span class="t-label t-muted">{{ session.user.role }}</span>
      </div>
      <MvButton v-else kind="primary" @click="signIn">{{ t("auth.signInWithGitHub") }}</MvButton>
      <label class="mv-menu-row">
        <span class="t-label t-muted">{{ t("nav.language") }}</span>
        <MvSelect v-model="locale" :options="localeOptions" :width="180" />
      </label>
      <MvButton kind="text" :to="{ name: 'about' }" @click="open = false">
        {{ t("page-title.about") }}
      </MvButton>
      <a class="mv-menu-link t-body" :href="config.siteReportUrl" target="_blank" rel="noopener">
        {{ t("page-title.bug-report") }}
      </a>
      <a class="mv-menu-link t-body" :href="config.sourceCodeUrl" target="_blank" rel="noopener">
        {{ t("page-title.source-code") }}
      </a>
      <template v-if="session.user">
        <hr class="mv-menu-rule" />
        <MvButton kind="text" @click="signOut">{{ t("auth.signOut") }}</MvButton>
        <MvButton kind="text" @click="askDelete">{{ t("auth.deleteAccount") }}</MvButton>
      </template>
    </div>

    <MvModal :open="confirming" :labelledby="confirmTitleId" @close="closeConfirm">
      <div class="mv-menu-confirm">
        <h2 :id="confirmTitleId" class="t-heading mv-menu-confirm-title">
          {{ t("auth.deleteTitle") }}
        </h2>
        <p class="t-body">{{ t("auth.deleteBody") }}</p>
        <MvError
          v-if="deleteFailed"
          :label="t('auth.deleteFailed') as string"
          :message="t('auth.deleteFailedMessage') as string"
        />
        <div class="mv-menu-confirm-actions">
          <MvButton kind="ghost" @click="closeConfirm">{{ t("auth.cancel") }}</MvButton>
          <MvButton kind="primary" :disabled="deleting" @click="confirmDelete">
            {{ t("auth.deleteConfirm") }}
          </MvButton>
        </div>
      </div>
    </MvModal>
  </div>
</template>

<style scoped>
.mv-menu {
  position: relative;
}
.mv-menu-panel {
  position: absolute;
  top: calc(100% + var(--s-2));
  right: 0;
  z-index: 30;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--s-1);
  min-width: 212px;
  padding: var(--s-4);
  background: var(--bg-panel);
  border: var(--line-w) solid var(--line);
  border-radius: var(--r);
}
.mv-menu-avatar {
  width: var(--control-h);
  height: var(--control-h);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  border: var(--line-w) solid var(--line);
  border-radius: 50%;
  overflow: hidden;
  background: var(--bg-sunken);
  color: var(--fg);
  cursor: pointer;
}
.mv-menu-avatar img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.mv-menu-account {
  display: flex;
  flex-direction: column;
  margin-bottom: var(--s-3);
}
.mv-menu-rule {
  align-self: stretch;
  margin: var(--s-2) 0;
  border: none;
  border-top: var(--line-w) solid var(--line);
}
.mv-menu-confirm {
  padding: var(--s-6);
}
.mv-menu-confirm-title {
  margin: 0 0 var(--s-3);
}
.mv-menu-confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--s-3);
  margin-top: var(--s-6);
}
.mv-menu-row {
  display: flex;
  flex-direction: column;
  gap: var(--s-1);
  margin-bottom: var(--s-2);
}
.mv-menu-link {
  min-height: var(--control-h);
  display: flex;
  align-items: center;
  color: var(--fg);
  text-decoration: none;
}
.mv-menu-link:hover {
  text-decoration: underline;
  text-underline-offset: 3px;
}
</style>
