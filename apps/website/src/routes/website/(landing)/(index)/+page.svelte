<script lang="ts">
  import { Helmet } from '@typie/ui/components';
  import CallToAction from './CallToAction.svelte';
  import Features from './Features.svelte';
  import Hero from './Hero.svelte';
  import Testimonials from './Testimonials.svelte';
</script>

<Helmet
  description="작성, 정리, 공유까지. 글쓰기의 모든 과정을 하나의 도구로 해결하세요."
  title="타이피 - 언제든 이어 쓰는 글쓰기 앱"
  trailing={null}
/>

<Hero />

<Features />

<Testimonials />

<CallToAction />
