<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { clamp } from '@typie/ui/utils';
  import Caret from '$lib/components/Caret.svelte';
  import { words } from '$lib/text';
  import { ramp, WRITE_LEAD_RANGE } from '../../scrub';
  import SceneLead from '../SceneLead.svelte';
  import { LAPTOP, lerpDevice, PHONE, TYPE_FIRST_RANGE, TYPE_SECOND_RANGE, TYPE_SPAN } from './device';
  import DeviceChrome from './DeviceChrome.svelte';
  import DeviceFrame from './DeviceFrame.svelte';
  import type { SceneProps } from '../../scrub';

  let { progress, cardsProgress = 0, lead, cards }: SceneProps = $props();

  const first = words(
    '나는 그때를 새로운 별이 탄생하는 순간처럼 기억해. 내가 콜로니 한켠에 있는 커다란 구멍으로 바깥을 내다보고 있었고, 그 원의 주변을 둘러 너를 기억하던 이들이 조화를 한 송이씩 내려놓던 그때.',
  );
  const second = words('블랙홀 같은 유리장을 빤히 바라보고 있자니 그 너머로 두터운 우주복을 입은 사람 한 명이 나타났었어.');

  const typed = (window: readonly [number, number], index: number, length: number) => {
    const at = window[0] + (index / length) * (window[1] - window[0]);
    return ramp(progress, at, at + TYPE_SPAN);
  };

  const lastTyped = (window: readonly [number, number], length: number) =>
    clamp(Math.ceil(((progress - window[0]) / (window[1] - window[0])) * length) - 1, -1, length - 1);

  const revealFirst = (index: number) => typed(TYPE_FIRST_RANGE, index, first.length);
  const revealSecond = (index: number) => typed(TYPE_SECOND_RANGE, index, second.length);

  const lastFirst = $derived(lastTyped(TYPE_FIRST_RANGE, first.length));
  const lastSecond = $derived(lastTyped(TYPE_SECOND_RANGE, second.length));
  const caretFirst = $derived(lastSecond >= 0 ? -1 : lastFirst);

  const t = $derived(ramp(progress, 0.38, 0.62));
  const leadIn = $derived(ramp(progress, ...WRITE_LEAD_RANGE));
  const device = $derived(lerpDevice(LAPTOP, PHONE, t));

  const wordClass = css({ position: 'relative', willChange: 'opacity, top' });
</script>

{#snippet caret()}
  <span class={css({ position: 'relative', display: 'inline-block' })}><Caret /></span>
{/snippet}

<DeviceFrame anchor={LAPTOP} {device} target={PHONE}>
  {#snippet aside()}
    <SceneLead {cards} {cardsProgress} {lead} shown={leadIn} />
  {/snippet}

  {#snippet decorations()}
    <DeviceChrome {device} {progress} {t} />
  {/snippet}

  {#if caretFirst < 0 && lastSecond < 0}
    <span class={wordClass}>{@render caret()}</span>
  {/if}
  {#each first as word, index (index)}
    {@const reveal = revealFirst(index)}
    <span style:opacity={reveal} style:top="{(1 - reveal) * 0.4}em" class={wordClass}>
      {word}{#if index === caretFirst}{@render caret()}{/if}{index < first.length - 1 || lastSecond >= 0 ? ' ' : ''}
    </span>
  {/each}
  {#each second as word, index (index)}
    {@const reveal = revealSecond(index)}
    {#if reveal > 0}
      <span style:opacity={reveal} style:top="{(1 - reveal) * 0.4}em" class={wordClass}>
        {word}{#if index === lastSecond}{@render caret()}{/if}{index < second.length - 1 ? ' ' : ''}
      </span>
    {/if}
  {/each}
</DeviceFrame>
