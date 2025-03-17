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
  description="창작자가 기다려온 글쓰기 앱을 만나보세요. 지금 얼리버드 한정 특별가로 미리 등록할 수 있어요!"
  image={{ src: 'https://cdn.glttr.io/opengraph/default.png', size: 'large' }}
  title="몰입해서 쓰고, 유연하게 공유하고, 깔끔하게 정리하는 스마트한 에디터, 글리터"
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
    글리터 사전 등록이 완료되었어요!

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
