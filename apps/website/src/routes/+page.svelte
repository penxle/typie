<script lang="ts">
  import { Helmet } from '$lib/components';
  import { css } from '$styled-system/css';
  import CtaSection from './CtaSection.svelte';
  import EditorSection from './EditorSection.svelte';
  import FaqSection from './FaqSection.svelte';
  import FocusSection from './FocusSection.svelte';
  import HeroSection from './HeroSection.svelte';
  import ShareSection from './ShareSection.svelte';
</script>

<Helmet description="글리터" title="글리터" trailing={null} />

<div class={css({ color: '[#282738]', backgroundColor: '[#FFFDF8]' })}>
  <HeroSection />

  <EditorSection />

  <FocusSection />

  <ShareSection />

  <CtaSection />

  <FaqSection />
</div>
