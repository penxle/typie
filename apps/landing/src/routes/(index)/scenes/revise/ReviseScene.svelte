<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { smootherstep } from '@typie/ui/utils';
  import PrismObject from '$lib/components/PrismObject.svelte';
  import { ramp, SCENE_LEAD_RANGE, SCENE_SLIDE_RANGE } from '../../scrub';
  import SceneLead from '../SceneLead.svelte';
  import { DIALOGUE_WINDOWS, TURNS } from './dialogue';
  import PrismComposer from './PrismComposer.svelte';
  import PrismNoteCard from './PrismNoteCard.svelte';
  import PrismRow from './PrismRow.svelte';
  import PrismStream from './PrismStream.svelte';
  import PrismToolRun from './PrismToolRun.svelte';
  import type { SceneProps } from '../../scrub';

  let { progress, cardsProgress = 0, lead, cards }: SceneProps = $props();

  const WELCOME = '어떤 이야기를 새롭게 바라볼까요?';

  let leadWidth = $state(0);
  let stackElement = $state<HTMLElement>();
  let leadTop = $state(0);

  $effect(() => {
    const element = stackElement;
    if (!element) return;

    const measure = () => {
      leadTop = element.offsetTop;
    };

    const observer = new ResizeObserver(measure);
    observer.observe(element);
    if (element.offsetParent) observer.observe(element.offsetParent);
    measure();

    return () => observer.disconnect();
  });

  const slide = $derived(ramp(progress, ...SCENE_SLIDE_RANGE));
  const leadIn = $derived(ramp(progress, ...SCENE_LEAD_RANGE));

  const typing = $derived(DIALOGUE_WINDOWS.findIndex((window) => progress < window.question));
  const composerText = $derived.by(() => {
    if (typing < 0) return '';
    const window = DIALOGUE_WINDOWS[typing];
    const question = TURNS[typing].question;
    return question.slice(0, Math.floor(ramp(progress, window.type[0], window.type[1]) * question.length));
  });

  const FOG = 24;
  const FOG_SAMPLES = 6;
  const fogMask = `linear-gradient(to bottom, ${Array.from({ length: FOG_SAMPLES + 1 }, (_, index) => {
    const progress = index / FOG_SAMPLES;
    return `rgb(0 0 0 / ${smootherstep(progress)}) ${progress * FOG}px`;
  }).join(', ')}, black ${FOG}px)`;

  const hostClass = css({
    display: 'grid',
    gridTemplateRows: '[minmax(0, 1fr)]',
    gridTemplateColumns: { lg: '[minmax(0, 557px) minmax(0, 600px)]' },
    justifyContent: 'center',
    gap: { base: '0', lg: '56px' },
    width: 'full',
    height: 'full',
  });
  const leadHostClass = css({
    display: { base: 'none', lg: 'block' },
    gridColumn: { lg: '1' },
    gridRow: { lg: '1' },
    alignSelf: 'start',
    paddingTop: '[var(--lead-top)]',
    minWidth: '0',
  });
  const stageHostClass = css({
    gridColumn: { lg: '2' },
    gridRow: { lg: '1' },
    display: 'grid',
    gridTemplateRows: '[minmax(0, 1fr)]',
    minWidth: '0',
    minHeight: '0',
    '--slide': { base: '0px', lg: '[calc((var(--lead-width) + 56px) / 2)]' },
    transform: '[translateX(calc((var(--slide-progress) - 1) * var(--slide)))]',
    willChange: 'transform',
  });
  const stageClass = css({
    display: 'flex',
    flexDirection: 'column',
    justifyContent: 'center',
    height: 'full',
    minHeight: '0',
    maxWidth: '[600px]',
    width: 'full',
    marginX: 'auto',
  });
  const stackClass = css({
    position: 'relative',
    display: 'grid',
    gridTemplateRows: '[minmax(0, 1fr)]',
    flex: '1',
    minHeight: '0',
    maxHeight: '[480px]',
    overflow: 'hidden',
    '& > *': { gridArea: '[1 / 1]', minHeight: '0' },
  });
  const welcomeClass = flex({
    direction: 'column',
    align: 'center',
    justify: 'center',
    gap: '0',
    textAlign: 'center',
    pointerEvents: 'none',
    transition: '[opacity 320ms ease, transform 320ms cubic-bezier(0.23, 1, 0.32, 1)]',
    '&[data-gone="true"]': { opacity: '0', transform: '[translateY(-12px)]' },
    _motionReduce: { transition: '[none]' },
  });
  const greetingClass = css({ fontFamily: 'ui', fontSize: { base: '17px', lg: '20px' }, lineHeight: '[1.5]', color: 'text.muted' });
  const threadClass = flex({ direction: 'column', justify: 'flex-end', paddingBottom: '8px' });
  const bubbleClass = css({
    maxWidth: '[80%]',
    paddingX: '16px',
    paddingY: '11px',
    borderRadius: '16px',
    backgroundColor: 'surface.inset',
    fontFamily: 'ui',
    fontSize: '15px',
    lineHeight: '[1.55]',
    color: 'text.default',
    wordBreak: 'keep-all',
  });
</script>

<div style:--lead-width="{leadWidth}px" style:--slide-progress={slide} class={hostClass}>
  <div style:--lead-top="{leadTop}px" class={leadHostClass} bind:clientWidth={leadWidth}>
    <SceneLead {cards} {cardsProgress} {lead} shown={leadIn} />
  </div>

  <div class={stageHostClass}>
    <div class={stageClass}>
      <div bind:this={stackElement} class={stackClass}>
        <div class={welcomeClass} data-gone={progress >= DIALOGUE_WINDOWS[0].question}>
          <div class={css({ marginTop: '[64px]', marginBottom: '[-38px]' })}>
            <PrismObject prismSize={108} target="prism" />
          </div>
          <p class={greetingClass}>{WELCOME}</p>
        </div>

        <div style:mask-image={fogMask} class={threadClass}>
          {#each TURNS as turn, index (index)}
            {@const window = DIALOGUE_WINDOWS[index]}

            <PrismRow shown={progress >= window.question}>
              <div class={flex({ justify: 'flex-end' })}>
                <div class={bubbleClass}>{turn.question}</div>
              </div>
            </PrismRow>

            <PrismRow shown={progress >= window.tools}>
              <PrismToolRun labels={turn.tools} shown={progress >= window.tools} />
            </PrismRow>

            {#each turn.reply as block, blockIndex (blockIndex)}
              <PrismRow shown={progress >= window.blocks[blockIndex]}>
                {#if block.kind === 'note'}
                  <PrismNoteCard entity={block.entity} lines={block.lines} time={block.time} title={block.title} />
                {:else}
                  <PrismStream prose={block.kind === 'quote'} shown={progress >= window.blocks[blockIndex]} text={block.text} />
                {/if}
              </PrismRow>
            {/each}
          {/each}
        </div>
      </div>

      <div class={css({ marginTop: '24px' })}>
        <PrismComposer text={composerText} />
      </div>
    </div>
  </div>
</div>
