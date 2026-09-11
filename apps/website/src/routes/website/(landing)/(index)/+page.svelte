<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Helmet } from '@typie/ui/components';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { clamp } from '@typie/ui/utils';
  import Lenis from 'lenis';
  import { untrack } from 'svelte';
  import { hydrateQuery } from '$lib/graphql';
  import { HEADER_FLOATS_AFTER } from '$lib/landing/components/header';
  import Header from '$lib/landing/components/Header.svelte';
  import Closing from './closing/Closing.svelte';
  import { FEATURES } from './features';
  import Hero from './hero/Hero.svelte';
  import FeatureCards from './scenes/FeatureCards.svelte';
  import SceneLead from './scenes/SceneLead.svelte';
  import { mobileSectionDvh, SCENE_SPAN } from './scrub';
  import type { PageData } from './$types';

  type Props = { data: PageData };

  let { data }: Props = $props();

  const query = $derived(hydrateQuery(() => data.query));

  type Span = { top: number; range: number };

  let scroller = $state<HTMLElement>();
  let content = $state<HTMLElement>();
  let closingSection = $state<HTMLElement>();
  let closingProgress = $state(0);
  let scrollTop = $state(0);
  let pageProgress = $state(0);
  let wide = $state(true);
  let sections = $state<HTMLElement[]>([]);
  let progress = $state<number[]>(FEATURES.map(() => 0));
  let pending = 0;

  let pageRange = 0;
  let closingSpan: Span | null = null;
  let sectionSpans: (Span | null)[] = [];

  const reduced = $derived(prefersReducedMotion.current);

  // 진행 구간이 반 화면도 안 되면 스크럽할 값이 없다 — 스크럽을 끄고 완료 상태로 둔다
  const spanOf = (element: HTMLElement, viewport: number): Span => {
    const range = element.offsetHeight - viewport;
    return { top: element.offsetTop, range: range > viewport / 2 ? range : 0 };
  };
  const progressIn = (span: Span, top: number) => (span.range > 0 ? clamp((top - span.top) / span.range, 0, 1) : 1);

  const paint = (top: number) => {
    pageProgress = pageRange > 0 ? clamp(top / pageRange, 0, 1) : 1;
    if (closingSpan) closingProgress = progressIn(closingSpan, top);
    for (const [index, span] of sectionSpans.entries()) progress[index] = span ? progressIn(span, top) : 0;
  };

  const apply = (element: HTMLElement) => {
    const top = element.scrollTop;
    scrollTop = top;
    paint(top);
  };

  const remeasure = () => {
    const element = scroller;
    if (!element) return;
    const viewport = element.clientHeight;
    pageRange = element.scrollHeight - viewport;
    closingSpan = closingSection ? spanOf(closingSection, viewport) : null;
    sectionSpans = sections.map((section) => (section ? spanOf(section, viewport) : null));

    apply(element);
  };

  const measure = () => {
    pending = 0;
    if (scroller) apply(scroller);
  };

  const onscroll = () => {
    if (pending === 0) pending = requestAnimationFrame(measure);
  };

  $effect(() => {
    const element = scroller;
    const inner = content;
    if (!element || !inner) return;

    const observer = new ResizeObserver(remeasure);
    observer.observe(element);
    observer.observe(inner);
    for (const section of sections) if (section) observer.observe(section);
    if (closingSection) observer.observe(closingSection);

    untrack(remeasure);

    return () => {
      observer.disconnect();
      if (pending > 0) cancelAnimationFrame(pending);
      pending = 0;
    };
  });

  // 마우스 휠은 노치당 델타가 커서 스크럽이 뚝뚝 끊긴다 — 스크롤 위치 자체를 프레임마다 보간한다.
  // 터치 기기는 하드웨어가 이미 관성을 주므로 네이티브에 맡긴다.
  $effect(() => {
    const wrapper = scroller;
    const inner = content;
    if (!wrapper || !inner) return;
    if (window.matchMedia('(pointer: coarse)').matches) return;

    const instance = new Lenis({ wrapper, content: inner, lerp: 0.14, autoRaf: true });

    return () => instance.destroy();
  });

  $effect(() => {
    const query = window.matchMedia('(min-width: 1024px)');
    const update = () => (wide = query.matches);
    update();
    query.addEventListener('change', update);
    return () => query.removeEventListener('change', update);
  });

  const progressOf = (index: number) => (reduced ? 1 : progress[index]);

  const sceneProgress = (value: number, end: number) => (wide ? clamp(value / SCENE_SPAN, 0, 1) : value * end);
  const cardsProgress = (value: number) => (wide ? clamp((value - SCENE_SPAN) / (1 - SCENE_SPAN), 0, 1) : 0);

  const scrollerClass = css({
    position: 'fixed',
    inset: '0',
    overflowY: 'auto',
    scrollbar: 'hidden',
    backgroundColor: 'surface.canvas',
    color: 'text.default',
  });
  const sectionClass = css({ position: 'relative', height: { base: '[calc(var(--section-dvh) * 1dvh)]', lg: '[440dvh]' } });
  const stageClass = css({
    position: 'sticky',
    top: '0',
    height: '[100dvh]',
    display: 'grid',
    gridTemplateRows: '[minmax(0, 1fr)]',
    paddingX: { base: '20px', lg: '64px' },
    paddingTop: '96px',
    paddingBottom: '48px',
  });
  const bodyClass = css({ position: 'relative', display: 'grid', gridTemplateRows: '[minmax(0, 1fr)]', minHeight: '0' });
  const tailClass = css({
    display: { base: 'grid', lg: 'none' },
    rowGap: '56px',
    paddingX: '20px',
    paddingTop: '48px',
    paddingBottom: '80px',
  });
  const railClass = css({
    position: 'fixed',
    top: '[50%]',
    right: '24px',
    width: '2px',
    height: '[160px]',
    borderRadius: 'full',
    backgroundColor: 'text.default/15',
    transform: '[translateY(-50%)]',
    overflow: 'hidden',
    display: { base: 'none', lg: 'block' },
  });
  const railFillClass = css({ width: 'full', height: 'full', backgroundColor: 'text.default', transformOrigin: '[top center]' });
</script>

<Helmet
  description="작성, 정리, 공유까지. 글쓰기의 모든 과정을 하나의 도구로 해결하세요."
  title="타이피 - 언제든 이어 쓰는 글쓰기 앱"
  trailing={null}
/>

<div bind:this={scroller} class={scrollerClass} {onscroll}>
  <Header floating={scrollTop > HEADER_FLOATS_AFTER} />

  <div bind:this={content}>
    <Hero landingStats$key={query.data.landingStats} {scrollTop} />

    {#each FEATURES as feature, index (feature.id)}
      {@const value = progressOf(index)}
      {@const Scene = feature.scene}
      <section bind:this={sections[index]} style:--section-dvh={mobileSectionDvh(feature.sceneEnd)} class={sectionClass}>
        <div class={stageClass}>
          <div class={bodyClass}>
            <Scene
              cards={feature.cards}
              cardsProgress={cardsProgress(value)}
              lead={feature.lead}
              progress={sceneProgress(value, feature.sceneEnd)}
            />
          </div>
        </div>
      </section>

      <div class={tailClass}>
        <SceneLead lead={feature.lead} shown={1} />
        <FeatureCards cards={feature.cards} progress={1} />
      </div>
    {/each}

    <div bind:this={closingSection}><Closing progress={reduced ? 1 : closingProgress} /></div>
  </div>

  <div class={railClass}>
    <div style:transform="scaleY({pageProgress})" class={railFillClass}></div>
  </div>
</div>
