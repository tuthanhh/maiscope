<script setup lang="ts">
import { ref, useId } from "vue";
import { useRoute } from "vue-router";
import useI18n from "~/composables/useI18n";
import useDisclosure from "~/composables/useDisclosure";
import { useSessionStore } from "~/stores/session";
import MvButton from "~/components/ui/MvButton.vue";
import MvError from "~/components/ui/MvError.vue";
import MvModal from "~/components/ui/MvModal.vue";

// The account, on its own at the end of the top bar (frontend-redesign 21):
// "Sign in" while signed out; signed in, the avatar opens the login and role,
// Sign out, and Delete account. Settings live behind the gear beside it.
const { t } = useI18n();
const route = useRoute();
const session = useSessionStore();
const { root, open, panelId, onFocusOut, focusTrigger } = useDisclosure();

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
// The panel closed under the trigger that opened it, so the dialog has
// nothing to return focus to; give it back to the trigger (or, once signed
// out, to whatever replaced it — the Sign in button carries the same class).
function closeConfirm(): void {
  confirming.value = false;
  requestAnimationFrame(focusTrigger);
}
</script>

<template>
  <div ref="root" class="mv-account" @focusout="onFocusOut">
    <!-- Hidden until a stored session is checked, so a signed-in reload does
         not flash "Sign in". -->
    <template v-if="session.ready && !session.user">
      <MvButton kind="ghost" class="mv-menu-trigger" @click="session.signIn(route.fullPath)">
        {{ t("auth.signIn") }}
      </MvButton>
    </template>
    <template v-else-if="session.user">
      <button
        type="button"
        class="mv-menu-trigger mv-account-avatar"
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
      <div v-if="open" :id="panelId" class="mv-account-panel">
        <div class="mv-account-who">
          <span class="t-strong">{{ session.user.login }}</span>
          <span class="t-label t-muted">{{ session.user.role }}</span>
        </div>
        <button type="button" class="mv-account-action t-body" @click="signOut">
          {{ t("auth.signOut") }}
        </button>
        <button type="button" class="mv-account-action t-body" @click="askDelete">
          {{ t("auth.deleteAccount") }}
        </button>
      </div>
    </template>

    <MvModal :open="confirming" :labelledby="confirmTitleId" @close="closeConfirm">
      <div class="mv-account-confirm">
        <h2 :id="confirmTitleId" class="t-heading mv-account-confirm-title">
          {{ t("auth.deleteTitle") }}
        </h2>
        <p class="t-body">{{ t("auth.deleteBody") }}</p>
        <MvError
          v-if="deleteFailed"
          :label="t('auth.deleteFailed') as string"
          :message="t('auth.deleteFailedMessage') as string"
        />
        <div class="mv-account-confirm-actions">
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
.mv-account {
  position: relative;
}
.mv-account-avatar {
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
.mv-account-avatar img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.mv-account-panel {
  position: absolute;
  top: calc(100% + var(--s-2));
  right: 0;
  z-index: 30;
  display: flex;
  flex-direction: column;
  min-width: 200px;
  padding: var(--s-4);
  background: var(--bg-panel);
  border: var(--line-w) solid var(--line);
  border-radius: var(--r);
}
.mv-account-who {
  display: flex;
  flex-direction: column;
  padding-bottom: var(--s-3);
  margin-bottom: var(--s-2);
  border-bottom: var(--line-w) solid var(--line);
}
.mv-account-action {
  min-height: var(--control-h);
  padding: 0;
  border: none;
  background: none;
  color: var(--fg);
  text-align: left;
  cursor: pointer;
}
.mv-account-action:hover {
  text-decoration: underline;
  text-underline-offset: 3px;
}
.mv-account-confirm {
  padding: var(--s-6);
}
.mv-account-confirm-title {
  margin: 0 0 var(--s-3);
}
.mv-account-confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--s-3);
  margin-top: var(--s-6);
}
</style>
