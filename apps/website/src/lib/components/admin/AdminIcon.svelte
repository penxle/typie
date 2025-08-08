<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Icon } from '$lib/components';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Component } from 'svelte';
  import type { SVGAttributes } from 'svelte/elements';

  type Props = {
    icon: Component;
    style?: SystemStyleObject;
    size?: 12 | 14 | 16 | 18 | 20 | 24 | 28 | 32;
  } & Omit<SVGAttributes<SVGSVGElement>, 'style' | 'size'>;

  let { icon, style, size = 16, ...rest }: Props = $props();
</script>

<Icon style={css.raw({ transition: '[none]' }, style)} {icon} {size} {...rest} />
