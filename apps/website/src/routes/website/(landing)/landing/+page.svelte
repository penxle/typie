<script lang="ts">
  import { onMount } from 'svelte';
  import CallToAction from './CallToAction.svelte';
  import Features from './Features.svelte';
  import Hero from './Hero.svelte';
  import Testimonials from './Testimonials.svelte';

  onMount(() => {
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

    const elementsToObserve = document.querySelectorAll('[data-observe]');
    elementsToObserve.forEach((element) => observer.observe(element));

    return () => {
      elementsToObserve.forEach((element) => observer.unobserve(element));
    };
  });
</script>

<Hero />

<Features />

<Testimonials />

<CallToAction />
