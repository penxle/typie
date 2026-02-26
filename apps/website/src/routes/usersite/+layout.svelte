<script lang="ts">
  import { onMount } from 'svelte';
  import { pollBootstrapAssertion } from '$lib/bootstrap';

  let { children } = $props();

  onMount(pollBootstrapAssertion);
</script>

{@render children?.()}
