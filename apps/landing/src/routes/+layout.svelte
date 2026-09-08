<script lang="ts">
  import '../app.css';

  import { css } from '@typie/styled-system/css';
  import type { Snippet } from 'svelte';

  type Props = { children: Snippet };

  let { children }: Props = $props();
</script>

<div class={css({ minHeight: '[100dvh]' })}>
  {@render children()}
</div>
