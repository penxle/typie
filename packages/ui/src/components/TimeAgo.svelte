<script lang="ts">
  import '@typie/lib/dayjs';

  import { css } from '@typie/styled-system/css';
  import dayjs from 'dayjs';
  import type { SystemStyleObject } from '@typie/styled-system/types';

  type Props = {
    timestamp: number;
    style?: SystemStyleObject;
  };

  let { timestamp, style }: Props = $props();

  let now = $state(Date.now());

  $effect(() => {
    const id = setInterval(() => (now = Date.now()), 60_000);
    return () => clearInterval(id);
  });

  const label = $derived((now, dayjs(timestamp).fromNow()));
</script>

<span class={css(style)}>{label}</span>
