<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import type { Snippet } from 'svelte';

  type Props = {
    children: Snippet;
  };

  let { children }: Props = $props();

  const cancel = (event: Event) => {
    event.preventDefault();
    event.stopPropagation();
  };
</script>

<svelte:window
  oncontextmenucapture={cancel}
  oncopycapture={cancel}
  oncutcapture={cancel}
  ondragstartcapture={cancel}
  onkeydowncapture={(event) => {
    const cmdKey = event.metaKey || event.ctrlKey;

    if (cmdKey && ['p', 's', 'u'].includes(event.key)) {
      cancel(event);
    }

    if (event.key === 'F12' || (cmdKey && (event.shiftKey || event.altKey) && ['i', 'c'].includes(event.key))) {
      cancel(event);
    }
  }}
  ontouchstartcapture={cancel}
/>

<div class={css({ display: 'contents' })} role="none">
  {@render children()}
</div>
