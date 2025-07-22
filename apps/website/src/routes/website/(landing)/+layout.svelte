<script lang="ts">
  import { css } from '$styled-system/css';
  import Footer from './Footer.svelte';
  import Header from './Header.svelte';

  const { children } = $props();
</script>

<div
  class={css({ width: '[100dvw]', minHeight: '[100dvh]', color: 'gray.900', backgroundColor: 'white', wordBreak: 'keep-all' })}
  data-element="root"
>
  <Header />

  <div class={css({ paddingTop: '96px' })}>
    {@render children()}
  </div>

  <Footer />
</div>
