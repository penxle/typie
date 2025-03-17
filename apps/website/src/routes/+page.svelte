<script lang="ts">
  import { Helmet } from '$lib/components';
  import { css } from '$styled-system/css';
  import CtaSection from './CtaSection.svelte';
  import EditorSection from './EditorSection.svelte';
  import FaqSection from './FaqSection.svelte';
  import FocusSection from './FocusSection.svelte';
  import Footer from './Footer.svelte';
  import HeroSection from './HeroSection.svelte';
  import ShareSection from './ShareSection.svelte';
</script>

<Helmet
  description="창작자가 기다려온 글쓰기 앱을 만나보세요. 지금 얼리버드 한정 특별가로 미리 등록할 수 있어요!"
  image={{ src: 'https://cdn.glttr.io/opengraph/default.png', size: 'large' }}
  title="몰입해서 쓰고, 유연하게 공유하고, 깔끔하게 정리하는 스마트한 에디터, 글리터"
  trailing={null}
/>

<div class={css({ color: '[#282738]', backgroundColor: '[#FFFDF8]' })}>
  <HeroSection />

  <EditorSection />

  <FocusSection />

  <ShareSection />

  <CtaSection />

  <FaqSection />

  <Footer />
</div>
