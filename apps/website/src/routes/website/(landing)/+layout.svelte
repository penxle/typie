<script lang="ts">
  import { css } from '$styled-system/css';
  import Footer from './Footer.svelte';
  import Header from './Header.svelte';

  const { children } = $props();

  let element = $state<HTMLDivElement>();
  let elements = $state<NodeListOf<Element>>();

  $effect(() => {
    if (!element) return;

    const observer = new MutationObserver(() => {
      elements = document.querySelectorAll('[data-observe]');
    });

    observer.observe(element, { childList: true });
    elements = document.querySelectorAll('[data-observe]');

    return () => {
      observer.disconnect();
    };
  });

  $effect(() => {
    if (!elements) return;

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

  <div bind:this={element} class={css({ paddingTop: '96px' })}>
    {@render children()}
  </div>

  <Footer />
</div>
