<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import useI18n from "~/composables/useI18n";
import usePageTitle from "~/composables/usePageTitle";
import { SignInError, useSessionStore } from "~/stores/session";
import MvButton from "~/components/ui/MvButton.vue";
import MvError from "~/components/ui/MvError.vue";

// Where GitHub sends the browser back (contract §4, step 3). The state must
// be the one this tab sent, or nothing is exchanged; then the code is traded
// for a session and the page replaces itself with where sign-in began, so
// Back does not return here with a spent code.
defineOptions({ name: "AuthCallbackPage" });

const { t } = useI18n();
const route = useRoute();
const router = useRouter();
const session = useSessionStore();
const failure = ref<"state" | "refused" | "unreachable" | "denied" | null>(null);

usePageTitle(() => ({ title: t("auth.signingIn") as string }));

const message = computed(() => {
  switch (failure.value) {
    case "state":
      return t("auth.failureState");
    case "refused":
      return t("auth.failureRefused");
    case "denied":
      return t("auth.failureDenied");
    default:
      return t("auth.failureUnreachable");
  }
});

onMounted(async () => {
  const { code, state, error } = route.query;
  // The user pressed Cancel on GitHub's page.
  if (typeof error === "string") {
    failure.value = "denied";
    return;
  }
  if (typeof code !== "string" || typeof state !== "string") {
    failure.value = "state";
    return;
  }
  try {
    const returnTo = await session.completeSignIn(code, state);
    await router.replace(returnTo);
  } catch (err) {
    failure.value = err instanceof SignInError ? err.kind : "unreachable";
  }
});

function retry(): void {
  session.signIn("/");
}
</script>

<template>
  <section class="mv-auth">
    <p v-if="!failure" class="t-body t-muted">{{ t("auth.signingIn") }}</p>
    <template v-else>
      <MvError
        :label="t('auth.failed') as string"
        :message="message as string"
      />
      <div class="mv-auth-actions">
        <MvButton kind="primary" @click="retry">{{ t("auth.tryAgain") }}</MvButton>
        <MvButton kind="ghost" :to="{ name: 'home' }">{{ t("auth.backHome") }}</MvButton>
      </div>
    </template>
  </section>
</template>

<style scoped>
.mv-auth {
  max-width: var(--modal-w);
  margin: 0 auto;
  padding: var(--s-12) var(--s-4);
  text-align: center;
}
.mv-auth-actions {
  display: flex;
  justify-content: center;
  gap: var(--s-3);
  margin-top: var(--s-6);
}
</style>
