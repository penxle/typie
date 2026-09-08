<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import GlobeIcon from '~icons/lucide/globe';
  import SmilePlusIcon from '~icons/lucide/smile-plus';
  import { words } from '$lib/landing/text';
  import { ramp, rise, SCENE_LEAD_RANGE, SCENE_SLIDE_RANGE } from '../../scrub';
  import SceneLead from '../SceneLead.svelte';
  import { ADDRESS, DOC_REVEAL_RANGE, DOC_SPAN, EMOJIS, REACTION, REACTION_SEQUENCE, SHARE_FLOW } from './share';
  import ShareRow from './ShareRow.svelte';
  import type { SceneProps } from '../../scrub';

  let { progress, cardsProgress = 0, lead, cards }: SceneProps = $props();

  const list = words(
    '나는 내 기억들에 대해 생각할 때마다 다른 모습을 떠올려. 발 디딜 틈도 없이 가득 찬 헛간을. 쌓이고 깎이며 굽이진 모래사장을. 새카맣도록 켜켜이 겹쳐진 셀로판을. 이끼 빛이 되도록 뒤섞인 팔레트를.',
  );
  const count = list.length;
  const reveal = (index: number) => {
    const at = (index / count) * DOC_REVEAL_RANGE[1];
    return ramp(progress, at, at + DOC_SPAN);
  };

  const ROW_GAP = 56;
  const LEAD_GAP = ROW_GAP + 40;

  let leadHeight = $state(0);

  const rowIn = $derived(ramp(progress, ...SHARE_FLOW.rowIn));
  const on = $derived(ramp(progress, ...SHARE_FLOW.on));
  const addressIn = $derived(ramp(progress, ...SHARE_FLOW.addressIn));
  const button = $derived(ramp(progress, ...SHARE_FLOW.button));
  const slide = $derived(ramp(progress, ...SCENE_SLIDE_RANGE));
  const leadIn = $derived(ramp(progress, ...SCENE_LEAD_RANGE));
  const pop = (index: number) =>
    ramp(progress, REACTION.start + index * REACTION.step, REACTION.start + REACTION.span + index * REACTION.step);

  const rootClass = css({ display: 'grid', gridTemplateRows: '[auto auto]', alignContent: 'center', width: 'full', height: 'full' });
  const leadHostClass = css({
    display: { base: 'none', lg: 'block' },
    width: 'full',
    maxWidth: '[960px]',
    marginX: 'auto',
    paddingBottom: '[var(--lead-gap)]',
  });
  const stageHostClass = css({
    display: 'grid',
    minHeight: '0',
    '--shift': '[calc((var(--slide-progress) - 1) * var(--lead-height) / 2)]',
    transform: '[translateY(var(--shift))]',
    willChange: 'transform',
  });
  const stageClass = css({ position: 'relative', width: 'full', maxWidth: '[960px]', marginX: 'auto' });
  const rowHostClass = css({ position: 'absolute', left: '0' });
  const addressClass = flex({
    position: 'absolute',
    right: '0',
    align: 'center',
    gap: '8px',
    height: '[40px]',
    fontFamily: 'landing',
    fontSize: { base: '15px', md: '20px', lg: '24px' },
    color: 'palette.blue',
    whiteSpace: 'nowrap',
    pointerEvents: 'none',
  });
  const textClass = css({
    fontFamily: 'prose',
    fontSize: { base: '18px', md: '24px', xl: '28px' },
    lineHeight: '[1.7]',
    letterSpacing: '[-0.01em]',
    color: 'text.default',
    wordBreak: 'keep-all',
  });
  const wordClass = css({ display: 'inline-block', marginRight: '[0.25em]', willChange: 'opacity, transform' });
  const reactionsClass = flex({ align: 'center', gap: '6px', marginTop: '28px', minHeight: '[36px]' });
  const buttonClass = css({ display: 'inline-flex', padding: '4px', borderRadius: '6px', color: 'text.muted' });
  const listClass = flex({ align: 'center', gap: '6px', marginLeft: '4px', wrap: 'wrap' });
  const itemClass = css({ display: 'inline-flex', willChange: 'opacity, transform' });
</script>

<div style:--lead-gap="{LEAD_GAP}px" style:--lead-height="{leadHeight}px" style:--slide-progress={slide} class={rootClass}>
  <div class={leadHostClass} bind:clientHeight={leadHeight}>
    <SceneLead {cards} {cardsProgress} {lead} shown={leadIn} split />
  </div>

  <div class={stageHostClass}>
    <div class={stageClass}>
      <div style:top="{-ROW_GAP}px" class={rowHostClass}><ShareRow enter={rowIn} {on} show={rowIn} /></div>
      <div style:top="{-ROW_GAP}px" style:opacity={addressIn} style:transform={rise(addressIn, 6)} class={addressClass}>
        <Icon icon={GlobeIcon} size={20} />
        <span>{ADDRESS}</span>
      </div>

      <div class={textClass}>
        {#each list as word, index (index)}
          {@const shown = reveal(index)}
          <span style:opacity={shown} style:transform={rise(shown, 0.4, 'em')} class={wordClass}>{word}</span>
        {/each}
      </div>

      <div class={reactionsClass}>
        <span style:opacity={button} class={buttonClass}><Icon icon={SmilePlusIcon} size={20} /></span>
        <span class={listClass}>
          {#each REACTION_SEQUENCE as emoji, index (index)}
            {@const shown = pop(index)}
            <span style:opacity={shown} style:transform="scale({0.5 + shown * 0.5})" class={itemClass}>
              <Icon icon={EMOJIS[emoji]} size={24} />
            </span>
          {/each}
        </span>
      </div>
    </div>
  </div>
</div>
