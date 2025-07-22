<script lang="ts">
  import { page } from '$app/state';
  import { css } from '$styled-system/css';
  import Footer from './Footer.svelte';
  import Header from './Header.svelte';

  const { children } = $props();

  $effect(() => {
    void page.url.pathname;

    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            entry.target.classList.add('in-view');
          }
        });
      },
      {
        threshold: 0.1,
        rootMargin: '0px 0px 50px 0px',
      },
    );

    const elements = document.querySelectorAll('[data-observe]');
    elements.forEach((element) => observer.observe(element));

    return () => {
      observer.disconnect();
    };
  });
</script>

<div
  class={css({ width: 'full', minHeight: '[100dvh]', color: 'gray.900', backgroundColor: 'white', wordBreak: 'keep-all' })}
  data-element="root"
>
  <Header />

  <div class={css({ paddingTop: '96px' })}>
    {@render children()}
  </div>

  <Footer />
</div>
