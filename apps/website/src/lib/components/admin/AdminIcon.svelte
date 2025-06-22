<script lang="ts">
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import type { Component } from 'svelte';
  import type { SVGAttributes } from 'svelte/elements';
  import type { SystemStyleObject } from '$styled-system/types';

  type Props = {
    icon: Component;
    style?: SystemStyleObject;
    size?: 12 | 14 | 16 | 18 | 20 | 24 | 28 | 32;
  } & Omit<SVGAttributes<SVGSVGElement>, 'style' | 'size'>;

  let { icon, style, size = 16, ...rest }: Props = $props();
</script>

<Icon style={css.raw({ transition: '[none]' }, style)} {icon} {size} {...rest} />
