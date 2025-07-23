<script lang="ts">
  import { afterNavigate } from '$app/navigation';
  import { css } from '$styled-system/css';
  import Footer from './Footer.svelte';
  import Header from './Header.svelte';

  const { children } = $props();

  let element = $state<HTMLDivElement>();
  let elements = $state<NodeListOf<Element>>();

  afterNavigate(() => {
    elements = document.querySelectorAll('[data-observe]');
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
  class={css({
    width: 'full',
    minHeight: '[100dvh]',
    color: 'gray.900',
    backgroundColor: 'white',
    fontFamily: 'Pretendard',
    wordBreak: 'keep-all',
  })}
  data-element="root"
>
  <Header />

  <div bind:this={element} class={css({ paddingTop: '96px' })}>
    {@render children()}
  </div>

  <Footer />
</div>
