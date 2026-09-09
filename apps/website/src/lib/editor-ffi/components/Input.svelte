<script lang="ts">
  import EditContextInput from './EditContextInput.svelte';
  import TextareaInput from './TextareaInput.svelte';

  const supportsEditContext = typeof EditContext === 'function';
</script>

{#if supportsEditContext}
  <EditContextInput />
{:else}
  <TextareaInput />
{/if}
