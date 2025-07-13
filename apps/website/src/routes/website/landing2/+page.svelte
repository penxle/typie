<script lang="ts">
  import { css } from '$styled-system/css';
  import CallToAction from './CallToAction.svelte';
  import DocumentFeatures from './DocumentFeatures.svelte';
  import Footer from './Footer.svelte';
  import Header from './Header.svelte';
  import Hero from './Hero.svelte';
  import Manifesto from './Manifesto.svelte';
  import Testimonials from './Testimonials.svelte';
  import TrackingFeatures from './TrackingFeatures.svelte';
  import WritingFeatures from './WritingFeatures.svelte';
</script>

<div
  class={css({ width: '[100dvw]', minHeight: '[100dvh]', color: 'gray.900', backgroundColor: 'gray.50', wordBreak: 'keep-all' })}
  data-element="root"
>
  <Header />

  <Hero />

  <Manifesto />

  <WritingFeatures />

  <DocumentFeatures />

  <TrackingFeatures />

  <Testimonials />

  <CallToAction />

  <Footer />
</div>
