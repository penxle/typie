<script lang="ts">
  import CheckIcon from '~icons/lucide/check';
  import MonitorIcon from '~icons/lucide/monitor';
  import MoonIcon from '~icons/lucide/moon';
  import SunIcon from '~icons/lucide/sun';
  import SunMoonIcon from '~icons/lucide/sun-moon';
  import { Icon, Menu, MenuItem } from '$lib/components';
  import { getThemeContext } from '$lib/context';
  import { css } from '$styled-system/css';
  import SidebarButton from './SidebarButton.svelte';
  import type { Component } from 'svelte';
  import type { Theme } from '$lib/context';

  const themes: Record<Theme, { icon: Component; label: string }> = {
    auto: { icon: MonitorIcon, label: '시스템 설정' },
    light: { icon: SunIcon, label: '라이트' },
    dark: { icon: MoonIcon, label: '다크' },
  };

  const themeNames: Theme[] = ['auto', 'light', 'dark'];

  const theme = getThemeContext();

  let open = $state(false);
</script>

<Menu offset={8} placement="right-start" bind:open>
  {#snippet button()}
    <SidebarButton icon={SunMoonIcon} label="테마 설정" />
  {/snippet}

  {#each themeNames as name (name)}
    <MenuItem
      icon={themes[name].icon}
      onclick={() => {
        theme.current = name;
      }}
    >
      {themes[name].label}

      {#if theme.current === name}
        <Icon style={css.raw({ marginLeft: 'auto', color: 'text.brand' })} icon={CheckIcon} size={14} />
      {/if}
    </MenuItem>
  {/each}
</Menu>
