<script lang="ts">
  import { css } from '@typie/styled-system/css';

  type Props = {
    renderAsOption?: boolean;
  };

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  let { renderAsOption }: Props = $props();
</script>

<div
  class={css({
    flexShrink: '0',
    width: '4px',
    alignSelf: 'stretch',
    backgroundColor: { base: 'gray.200', _dark: 'dark.gray.700' },
  })}
></div>
