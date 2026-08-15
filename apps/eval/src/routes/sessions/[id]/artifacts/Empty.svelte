<script lang="ts">
  import { css } from '@typie/styled-system/css';

  type Props = { text: string };
  const { text }: Props = $props();
</script>

<p class={css({ fontSize: '13px', color: 'text.faint' })}>{text}</p>
