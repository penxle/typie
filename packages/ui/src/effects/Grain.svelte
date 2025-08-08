<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { base64 } from 'rfc4648';
  import { encode } from '../utils';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Snippet } from 'svelte';

  type Props = {
    style?: SystemStyleObject;
    freq?: number;
    opacity?: number;
    seed?: number;
    children?: Snippet;
  };

  let { style, freq = 1.6, opacity = 1, seed = 2, children }: Props = $props();

  const url = $derived(
    `url(data:image/svg+xml;base64,${base64.stringify(
      encode(
        `<svg xmlns='http://www.w3.org/2000/svg' width='512' height='512'><filter id='grain'><feTurbulence type='fractalNoise' baseFrequency='${freq}' numOctaves='4' seed='${seed}' /><feColorMatrix type='saturate' values='0' /><feComponentTransfer> <feFuncA type='discrete' tableValues='0 ${opacity}' /></feComponentTransfer></filter><rect width='100%' height='100%' filter='url(#grain)'/></svg>`,
      ),
    )})`,
  );
</script>

<div
  style:background-image={url}
  style:opacity
  class={css({ backgroundRepeat: 'repeat', backgroundSize: '1024px 1024px', mixBlendMode: 'multiply' }, style)}
>
  {@render children?.()}
</div>
