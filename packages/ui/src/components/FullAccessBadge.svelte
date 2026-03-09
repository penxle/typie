<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import type { SystemStyleObject } from '@typie/styled-system/types';

  type Props = {
    style?: SystemStyleObject;
  };

  let { style }: Props = $props();
</script>

<span
  class={css(
    {
      fontSize: '9px',
      fontWeight: 'bold',
      lineHeight: '[1]',
      letterSpacing: '0.02em',
      color: 'accent.brand.default',
      borderWidth: '1px',
      borderColor: 'accent.brand.subtle',
      paddingX: '5px',
      paddingY: '3px',
      borderRadius: 'full',
      whiteSpace: 'nowrap',
    },
    style,
  )}
>
  FULL ACCESS
</span>
