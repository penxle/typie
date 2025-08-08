<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Icon } from '$lib/components';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Component } from 'svelte';

  type Props = {
    icon: Component;
    style?: SystemStyleObject;
  };

  let { icon, style }: Props = $props();
</script>

<Icon style={css.raw({ '& *': { strokeWidth: '[1.5px]' } }, style)} {icon} size={20} />
