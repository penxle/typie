<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon, Menu, MenuItem } from '@typie/ui/components';
  import { getThemeContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import CheckIcon from '~icons/lucide/check';
  import MonitorIcon from '~icons/lucide/monitor';
  import MoonIcon from '~icons/lucide/moon';
  import SunIcon from '~icons/lucide/sun';
  import SunMoonIcon from '~icons/lucide/sun-moon';
  import type { Theme } from '@typie/ui/context';
  import type { Component } from 'svelte';

  const themes: Record<Theme, { icon: Component; label: string }> = {
    auto: { icon: MonitorIcon, label: '시스템 설정' },
    light: { icon: SunIcon, label: '라이트' },
    dark: { icon: MoonIcon, label: '다크' },
  };

  const themeNames: Theme[] = ['auto', 'light', 'dark'];

  const theme = getThemeContext();

  let open = $state(false);
</script>

<Menu offset={8} placement="top-start" bind:open>
  {#snippet button()}
    <button
      class={center({
        borderRadius: '8px',
        size: '32px',
        color: 'text.faint',
        transition: 'common',
        _hover: {
          color: 'text.subtle',
          backgroundColor: 'surface.muted',
        },
        '&[aria-pressed="true"]': {
          color: 'text.subtle',
          backgroundColor: 'surface.muted',
        },
      })}
      aria-pressed={open}
      type="button"
      use:tooltip={{ message: '테마 설정', placement: 'top', offset: 8 }}
    >
      <Icon icon={SunMoonIcon} size={20} />
    </button>
  {/snippet}

  {#each themeNames as name (name)}
    <MenuItem
      icon={themes[name].icon}
      onclick={() => {
        mixpanel.track('switch_theme', { old: theme.current, new: name, via: 'theme_switch' });
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
