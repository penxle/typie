<script lang="ts">
  import { graphql } from '$graphql';
  import { Helmet } from '$lib/components';
  import CallToAction from './CallToAction.svelte';
  import Features from './Features.svelte';
  import Hero from './Hero.svelte';
  import Testimonials from './Testimonials.svelte';

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const query = graphql(`
    query IndexPage_Query {
      me {
        id
      }
    }
  `);
</script>

<Helmet
  description="작성, 정리, 공유까지. 글쓰기의 모든 과정을 하나의 도구로 해결하세요."
  title="타이피 - 작가를 위한 올인원 글쓰기 도구"
  trailing={null}
/>

<Hero />

<Features />

<Testimonials />

<CallToAction />
