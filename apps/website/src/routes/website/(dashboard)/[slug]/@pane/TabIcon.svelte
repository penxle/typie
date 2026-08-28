<script lang="ts">
  import { onDestroy } from 'svelte';
  import { setTabIcon } from '$lib/desktop';

  type Props = { icon: string; color?: string | null };

  let { icon, color = null }: Props = $props();

  const key = Symbol();

  $effect(() => {
    setTabIcon(key, { icon, color });
  });

  onDestroy(() => setTabIcon(key, null));
</script>
