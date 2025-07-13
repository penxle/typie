<script lang="ts">
  import { animate, mix, scroll } from 'motion';
  import { onMount } from 'svelte';
  import CompassIcon from '~icons/lucide/compass';
  import DraftingCompassIcon from '~icons/lucide/drafting-compass';
  import LayersIcon from '~icons/lucide/layers';
  import WorkflowIcon from '~icons/lucide/workflow';
  import { Icon } from '$lib/components';
  import { clamp } from '$lib/utils';
  import { css } from '$styled-system/css';
  import { Motion } from './motion.svelte';
  import type { Component } from 'svelte';

  const progress = new Motion(0);

  type ContentSegment = { type: 'text'; words: string[] } | { type: 'icon'; icon: Component };

  const line1: ContentSegment[] = [{ type: 'text', words: ['생각을', '적고,', '다듬고,', '나중에', '꺼내어', '쓰는', '일.'] }];

  const line2: ContentSegment[] = [
    { type: 'text', words: ['글을', '쓴다는', '건', '그', '모든', '단계를'] },
    { type: 'icon', icon: LayersIcon },
    { type: 'text', words: ['포함한다.'] },
  ];

  const line3: ContentSegment[] = [
    { type: 'text', words: ['처음과', '끝이', '자연스럽게', '이어질', '수', '있도록'] },
    { type: 'icon', icon: WorkflowIcon },
    { type: 'text', words: ['설계된', '환경.'] },
  ];

  const line4: ContentSegment[] = [
    { type: 'text', words: ['글이', '중심이', '되도록,', '도구는'] },
    { type: 'icon', icon: DraftingCompassIcon },
    { type: 'text', words: ['그', '흐름을', '따라가도록'] },
    { type: 'icon', icon: CompassIcon },
    { type: 'text', words: ['구성했다.'] },
  ];

  const countElements = (segments: ContentSegment[]) => {
    return segments.reduce((count, segment) => {
      if (segment.type === 'text') {
        return count + segment.words.length;
      } else {
        return count + 1;
      }
    }, 0);
  };

  const totalElements = countElements(line1) + countElements(line2) + countElements(line3) + countElements(line4) + 2; // +2 for citation lines

  const textMixer = mix('#545557', '#e4e4e6');
  const iconMixer = mix('#545557', '#f59e0b');

  const getElementColor = (index: number, isIcon = false) => {
    const p = clamp(progress.current * totalElements - index, 0, 1);
    return isIcon ? iconMixer(p) : textMixer(p);
  };

  type RenderedSegment =
    | { type: 'text'; data: { word: string; color: string }[]; key: string }
    | { type: 'icon'; icon: Component; color: string; key: string };

  const renderLine = (segments: ContentSegment[], startIndex: number): RenderedSegment[] => {
    let currentIndex = startIndex;
    return segments.map((segment, segmentIdx) => {
      if (segment.type === 'text') {
        const result = segment.words.map((word, wordIdx) => {
          const index = currentIndex + wordIdx;
          return { word, color: getElementColor(index) };
        });
        currentIndex += segment.words.length;
        return { type: 'text', data: result, key: `seg-${segmentIdx}` } as const;
      } else {
        const color = getElementColor(currentIndex, true);
        currentIndex += 1;
        return { type: 'icon', icon: segment.icon, color, key: `seg-${segmentIdx}` } as const;
      }
    });
  };

  onMount(() => {
    const element = document.querySelector('[data-element="manifesto"]') as HTMLElement;

    return scroll(animate(progress.value, 1, { ease: 'linear' }), {
      offset: ['start -10%', 'end 110%'],
      target: element,
    });
  });
</script>

<div class={css({ backgroundColor: 'gray.50' })}>
  <div class={css({ borderTopRadius: 'full', width: 'full', height: '50px', backgroundColor: 'dark.gray.950' })}></div>
</div>

<div
  class={css({ position: 'relative', width: 'full', height: '[200dvh]', backgroundColor: 'dark.gray.950', zIndex: '0' })}
  data-element="manifesto"
>
  <div
    class={css({
      position: 'sticky',
      top: '0',
      left: '0',
      right: '0',
      width: 'full',
      height: '[100dvh]',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
    })}
  >
    <div
      class={css({
        maxWidth: '[1000px]',
        fontSize: '[36px]',
        fontWeight: 'medium',
        fontFamily: 'Paperlogy',
        wordBreak: 'keep-all',
        lineHeight: '[1.6]',
        letterSpacing: '[-0.03em]',
      })}
    >
      <div class={css({ display: 'flex', flexWrap: 'wrap', alignItems: 'center' })}>
        {#each renderLine(line1, 0) as item (item.key)}
          {#if item.type === 'text'}
            {#each item.data as { word, color }, i (i)}
              <span style:color class={css({ marginRight: '[0.25em]' })}>
                {word}
              </span>
            {/each}
          {:else if item.type === 'icon'}
            <span style:color={item.color} class={css({ marginRight: '[0.25em]' })}>
              <Icon style={css.raw({ transitionProperty: 'none' })} icon={item.icon} size={28} />
            </span>
          {/if}
        {/each}
      </div>

      <div class={css({ display: 'flex', flexWrap: 'wrap', alignItems: 'center' })}>
        {#each renderLine(line2, countElements(line1)) as item (item.key)}
          {#if item.type === 'text'}
            {#each item.data as { word, color }, i (i)}
              <span style:color class={css({ marginRight: '[0.25em]' })}>
                {word}
              </span>
            {/each}
          {:else if item.type === 'icon'}
            <span style:color={item.color} class={css({ marginRight: '[0.25em]' })}>
              <Icon style={css.raw({ transitionProperty: 'none' })} icon={item.icon} size={28} />
            </span>
          {/if}
        {/each}
      </div>

      <div class={css({ height: '24px' })}></div>

      <div class={css({ display: 'flex', flexWrap: 'wrap', alignItems: 'center' })}>
        {#each renderLine(line3, countElements(line1) + countElements(line2)) as item (item.key)}
          {#if item.type === 'text'}
            {#each item.data as { word, color }, i (i)}
              <span style:color class={css({ marginRight: '[0.25em]' })}>
                {word}
              </span>
            {/each}
          {:else if item.type === 'icon'}
            <span style:color={item.color} class={css({ marginRight: '[0.25em]' })}>
              <Icon style={css.raw({ transitionProperty: 'none' })} icon={item.icon} size={28} />
            </span>
          {/if}
        {/each}
      </div>

      <div class={css({ display: 'flex', flexWrap: 'wrap', alignItems: 'center' })}>
        {#each renderLine(line4, countElements(line1) + countElements(line2) + countElements(line3)) as item (item.key)}
          {#if item.type === 'text'}
            {#each item.data as { word, color }, i (i)}
              <span style:color class={css({ marginRight: '[0.25em]' })}>
                {word}
              </span>
            {/each}
          {:else if item.type === 'icon'}
            <span style:color={item.color} class={css({ marginRight: '[0.25em]' })}>
              <Icon style={css.raw({ transitionProperty: 'none' })} icon={item.icon} size={28} />
            </span>
          {/if}
        {/each}
      </div>

      <div class={css({ marginTop: '80px', textAlign: 'right' })}>
        <div
          style:color={getElementColor(countElements(line1) + countElements(line2) + countElements(line3) + countElements(line4))}
          class={css({ fontSize: '[18px]', fontWeight: 'medium', lineHeight: '[1.6]', fontStyle: 'italic' })}
        >
          — 「타이피 출시 선언문」 중에서
        </div>
        <div
          style:color={getElementColor(countElements(line1) + countElements(line2) + countElements(line3) + countElements(line4) + 1)}
          class={css({ fontSize: '[18px]', fontWeight: 'medium', lineHeight: '[1.6]', marginTop: '4px' })}
        >
          2025년 봄 기록됨
        </div>
      </div>
    </div>
  </div>
</div>

<div class={css({ backgroundColor: 'dark.gray.950' })}>
  <div class={css({ borderTopRadius: 'full', width: 'full', height: '50px', backgroundColor: 'gray.50' })}></div>
</div>
