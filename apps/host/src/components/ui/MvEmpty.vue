<script setup lang="ts">
// design-direction "Empty state": one sentence, centred, nothing else.
</script>

<template>
  <p class="mv-empty t-body t-muted"><slot /></p>
</template>

<style scoped>
.mv-empty {
  margin: 0;
  padding: var(--s-12) 0;
  text-align: center;
}
</style>
