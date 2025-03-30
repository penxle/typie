<script lang="ts">
  import { page } from '$app/state';
  import { Dialog, Helmet } from '$lib/components';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import CtaSection from './CtaSection.svelte';
  import EditorSection from './EditorSection.svelte';
  import FaqSection from './FaqSection.svelte';
  import FocusSection from './FocusSection.svelte';
  import Footer from './Footer.svelte';
  import HeroSection from './HeroSection.svelte';
  import ShareSection from './ShareSection.svelte';

  let confirmOpen = $state(false);

  $effect(() => {
    const message = page.url.searchParams.get('message');
    if (message) {
      alert(message);
    }
  });

  $effect(() => {
    if (page.url.searchParams.get('success') === '1') {
      confirmOpen = true;
    }
  });
</script>

<Helmet
  description="창작자가 기다려온 글쓰기 앱 타이피를 만나보세요. 기본적인 텍스트 편집은 물론, 다양한 꾸밈 요소와 글쓰기 편의 기능으로 작품의 완성도를 높이고 나만의 개성을 더할 수 있어요."
  image={{ src: 'https://typie.net/opengraph/default.png', size: 'large' }}
  struct={{ '@context': 'https://schema.org', '@type': 'WebSite', name: '타이피', alternateName: 'Typie', url: 'https://typie.co/' }}
  title="타이피 - 쓰고, 공유하고, 정리하는 글쓰기 공간"
  trailing={null}
/>

<div class={css({ wordBreak: 'keep-all', color: '[#282738]', backgroundColor: '[#FFFDF8]', overflow: 'hidden' })}>
  <HeroSection />

  <EditorSection />

  <FocusSection />

  <ShareSection />

  <CtaSection />

  <FaqSection />

  <Footer />
</div>

<Dialog bind:open={confirmOpen}>
  <form class={flex({ direction: 'column', align: 'center', gap: '20px', width: 'full' })} method="dialog">
    타이피 사전 등록이 완료되었어요!

    <button
      class={css({
        borderRadius: '8px',
        paddingX: '20px',
        paddingY: '12px',
        fontSize: '12px',
        color: 'white',
        backgroundColor: '[#4A2DA0]',
        width: 'full',
      })}
      type="submit"
    >
      감사합니다. 오픈일에 만나요!
    </button>
  </form>
</Dialog>
