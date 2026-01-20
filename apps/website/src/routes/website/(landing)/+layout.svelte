<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import Footer from './Footer.svelte';
  import Header from './Header.svelte';

  const { children } = $props();
</script>

<div
  class={css({
    width: 'full',
    minHeight: '[100dvh]',
    color: 'dark.gray.50',
    backgroundColor: 'dark.gray.950',
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
