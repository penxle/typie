<script lang="ts">
  import * as Sentry from '@sentry/sveltekit';
  import PrismPanelFailure from './PrismPanelFailure.svelte';
  import type { Snippet } from 'svelte';

  type Props = {
    children: Snippet;
  };

  let { children }: Props = $props();

  function handleError(error: unknown) {
    console.error(error);

    try {
      Sentry.captureException(error);
    } catch (err) {
      console.error(err);
    }
  }
</script>

<svelte:boundary onerror={handleError}>
  {@render children()}

  {#snippet failed(error, reset)}
    <PrismPanelFailure
      onRetry={() => {
        void error;
        reset();
      }}
    />
  {/snippet}
</svelte:boundary>
