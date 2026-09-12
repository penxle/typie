<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { tooltip } from '@typie/ui/actions';
  import { getThemeContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import { MediaQuery } from 'svelte/reactivity';
  import SunMoonIcon from './SunMoonIcon.svelte';
  import type { Placement } from '@floating-ui/dom';
  import type { SystemStyleObject } from '@typie/styled-system/types';

  type Props = {
    style?: SystemStyleObject;
    via: string;
    tooltipPlacement?: Placement;
  };

  let { style, via, tooltipPlacement = 'bottom' }: Props = $props();

  const theme = getThemeContext();
  const prefersDark = new MediaQuery('(prefers-color-scheme: dark)');

  const dark = $derived(theme.effectiveTheme === 'dark');
  const target = $derived(dark ? 'light' : 'dark');
  const label = $derived(target === 'dark' ? '다크 테마로 전환' : '라이트 테마로 전환');

  const flip = () => {
    const system = prefersDark.current ? 'dark' : 'light';
    const next = target === system ? 'auto' : target;
    mixpanel.track('switch_theme', { old: theme.currentTheme, new: next, via });
    theme.currentTheme = next;
  };
</script>

<button
  class={css(
    {
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      size: '32px',
      borderRadius: '8px',
      color: 'text.muted',
      transition: 'colors',
      _hover: { color: 'text.default' },
    },
    style,
  )}
  aria-label={label}
  aria-pressed={dark}
  onclick={flip}
  type="button"
  use:tooltip={{ message: label, placement: tooltipPlacement }}
>
  <SunMoonIcon {dark} />
</button>
