<script lang="ts">
  import { Helmet } from '$lib/components';
  import CallToAction from './CallToAction.svelte';
  import Features from './Features.svelte';
  import Hero from './Hero.svelte';
  import Testimonials from './Testimonials.svelte';
</script>

<Helmet description="본문 작성부터 아이디어 시각화까지, 글쓰기의 모든 과정을 지원합니다." title="글쓰기에 필요한 모든 것" />

<Hero />

<Features />

<Testimonials />

<CallToAction />
