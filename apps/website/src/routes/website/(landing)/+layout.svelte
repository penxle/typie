<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getThemeContext } from '@typie/ui/context';
  import Footer from './Footer.svelte';
  import Header from './Header.svelte';

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
    fontFamily: 'Pretendard',
    wordBreak: 'keep-all',
  })}
  data-element="root"
>
  <Header />

  <div class={css({ paddingTop: { sm: '56px', lg: '64px' } })}>
    {@render children()}
  </div>

  <Footer />
</div>
