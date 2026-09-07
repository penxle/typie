<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { getThemeContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import EclipseIcon from '~icons/lucide/eclipse';
  import MonitorIcon from '~icons/lucide/monitor';
  import MoonIcon from '~icons/lucide/moon';
  import SunIcon from '~icons/lucide/sun';
  import type { Theme } from '@typie/ui/context';
  import type { Component } from 'svelte';

  type Props = {
    via: string;
  };

  let { via }: Props = $props();

  const theme = getThemeContext();

  const options: { value: Theme; icon: Component; label: string }[] = [
    { value: 'auto', icon: MonitorIcon, label: '시스템 설정' },
    { value: 'light', icon: SunIcon, label: '라이트' },
    { value: 'dark', icon: MoonIcon, label: '다크' },
  ];
</script>

<div
  class={flex({
    alignItems: 'center',
    gap: '8px',
    paddingX: '8px',
    paddingTop: '6px',
    paddingBottom: '4px',
    fontSize: '13px',
    fontWeight: 'medium',
    color: 'text.default',
  })}
>
  <Icon style={css.raw({ flexShrink: '0', color: 'text.default' })} icon={EclipseIcon} size={14} />
  <span>테마</span>

  <div
    class={flex({ marginLeft: 'auto', alignItems: 'center', borderRadius: '6px', padding: '2px', backgroundColor: 'surface.inset' })}
    aria-label="테마"
    role="radiogroup"
  >
    {#each options as option (option.value)}
      <button
        class={center({
          borderRadius: '4px',
          size: '20px',
          color: theme.currentTheme === option.value ? 'text.default' : 'text.muted',
          backgroundColor: theme.currentTheme === option.value ? 'surface.active' : 'transparent',
          boxShadow: theme.currentTheme === option.value ? 'sm' : '[none]',
          transition: 'common',
          cursor: 'pointer',
          _hover: theme.currentTheme === option.value ? {} : { color: 'text.default' },
        })}
        aria-checked={theme.currentTheme === option.value}
        aria-label={option.label}
        onclick={() => {
          mixpanel.track('switch_theme', { old: theme.currentTheme, new: option.value, via });
          theme.currentTheme = option.value;
        }}
        role="radio"
        type="button"
        use:tooltip={{ message: option.label }}
      >
        <Icon icon={option.icon} size={12} />
      </button>
    {/each}
  </div>
</div>
