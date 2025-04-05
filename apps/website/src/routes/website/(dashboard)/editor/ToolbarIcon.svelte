<script lang="ts">
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import type { Component } from 'svelte';

  type Props = {
    icon: Component;
  };

  let { icon }: Props = $props();
</script>

<Icon style={css.raw({ '& *': { strokeWidth: '[1.5px]' } })} {icon} size={20} />
