<script lang="ts">
  import { css } from '$styled-system/css';
  import Footer from './Footer.svelte';
  import Header from './Header.svelte';

  const { children } = $props();
</script>

<div
  class={css({ width: '[100dvw]', minHeight: '[100dvh]', color: 'gray.900', backgroundColor: 'gray.50', wordBreak: 'keep-all' })}
  data-element="root"
>
  <Header />

  {@render children()}

  <Footer />
</div>
