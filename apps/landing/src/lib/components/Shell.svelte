<script lang="ts">
  import Footer from './Footer.svelte';
  import { HEADER_FLOATS_AFTER } from './header';
  import Header from './Header.svelte';
  import type { Snippet } from 'svelte';

  type Props = { children: Snippet };

  let { children }: Props = $props();

  let floating = $state(false);

  $effect(() => {
    const onscroll = () => (floating = window.scrollY > HEADER_FLOATS_AFTER);
    onscroll();
    window.addEventListener('scroll', onscroll, { passive: true });
    return () => window.removeEventListener('scroll', onscroll);
  });
</script>

<Header {floating} />
{@render children()}
<Footer />
