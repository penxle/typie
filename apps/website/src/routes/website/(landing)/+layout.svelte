<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getThemeContext } from '@typie/ui/context';

  const { children } = $props();

  const theme = getThemeContext();

  $effect(() => {
    theme.overrideTheme = 'dark';
    theme.overrideVariant = { light: 'white', dark: 'black' };

    return () => {
      theme.overrideTheme = undefined;
      theme.overrideVariant = undefined;
    };
  });
</script>

<div
  class={css({
    width: 'full',
    minHeight: '[100dvh]',
    color: 'text.default',
    backgroundColor: 'surface.canvas',
    fontFamily: 'landing',
    wordBreak: 'keep-all',
  })}
>
  {@render children()}
</div>
