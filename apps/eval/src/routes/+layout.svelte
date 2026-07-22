<script lang="ts">
  import '../app.css';

  import { NotificationProvider } from '@typie/ui/notification';
  import type { Snippet } from 'svelte';

  type Props = { children: Snippet };
  const { children }: Props = $props();
</script>

{@render children()}

<NotificationProvider />
