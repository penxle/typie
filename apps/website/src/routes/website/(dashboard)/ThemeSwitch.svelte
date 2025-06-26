<script lang="ts">
  import CheckIcon from '~icons/lucide/check';
  import MonitorIcon from '~icons/lucide/monitor';
  import MoonIcon from '~icons/lucide/moon';
  import SunIcon from '~icons/lucide/sun';
  import SunMoonIcon from '~icons/lucide/sun-moon';
  import { Icon, Menu, MenuItem } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { css } from '$styled-system/css';
  import SidebarButton from './SidebarButton.svelte';
  import type { Component } from 'svelte';
  import type { Theme } from '$lib/state/theme.svelte';

  const themes: Record<Theme, { icon: Component; label: string }> = {
    auto: { icon: MonitorIcon, label: '시스템 설정 따르기' },
    light: { icon: SunIcon, label: '라이트 모드' },
    dark: { icon: MoonIcon, label: '다크 모드' },
  };

  const themeOptions: Theme[] = ['auto', 'light', 'dark'];

  const app = getAppContext();

  let open = $state(false);
</script>

<Menu offset={8} placement="right-start" bind:open>
  {#snippet button()}
    <SidebarButton icon={SunMoonIcon} label="테마 설정" />
  {/snippet}

  {#each themeOptions as theme (theme)}
    <MenuItem icon={themes[theme].icon} onclick={() => (app.theme.current = theme)}>
      {themes[theme].label}

      {#if app.theme.current === theme}
        <Icon style={css.raw({ marginLeft: 'auto', color: 'text.brand' })} icon={CheckIcon} size={14} />
      {/if}
    </MenuItem>
  {/each}
</Menu>
