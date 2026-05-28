<script lang="ts">
  // v2 FFI: find/replace 미구현 — stub
  let { close }: { close: () => void } = $props();
</script>

<button hidden onclick={close} type="button">×</button>
