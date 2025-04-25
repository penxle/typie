<script lang="ts">
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import type { Component } from 'svelte';
  import type { SystemStyleObject } from '$styled-system/types';

  type Props = {
    icon: Component;
    style?: SystemStyleObject;
  };

  let { icon, style }: Props = $props();
</script>

<Icon style={css.raw({ '& *': { strokeWidth: '[1.5px]' } }, style)} {icon} size={20} />
