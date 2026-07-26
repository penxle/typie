<script lang="ts">
  import { css } from '@typie/styled-system/css';

  // 본문 오른쪽에 세우는 눈금자. 브라우저 스크롤바를 대신하면서, 그 자리에 지적 분포까지 얹는다.
  // 스크롤바와 나란히 두면 같은 일을 하는 막대가 둘이 되므로 원래 스크롤바는 감춘다.
  export type RailMark = { feedbackId: string; number: number; position: number; state: 'unseen' | 'seen' | 'fail' };

  type Props = {
    marks: RailMark[];
    // 본문 스크롤 창의 위치(0~1). 없으면 창을 그리지 않는다.
    viewport: { start: number; end: number } | null;
    onSelect: (feedbackId: string) => void;
    onSeek: (fraction: number) => void;
  };
  const { marks, viewport, onSelect, onSeek }: Props = $props();

  let trackEl = $state<HTMLElement | undefined>();

  const seekTo = (clientY: number) => {
    const el = trackEl;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    onSeek(Math.min(1, Math.max(0, (clientY - rect.top) / rect.height)));
  };

  const onPointerDown = (e: PointerEvent) => {
    if (e.currentTarget instanceof HTMLElement) {
      e.currentTarget.setPointerCapture(e.pointerId);
    }
    seekTo(e.clientY);
  };

  const onPointerMove = (e: PointerEvent) => {
    if (e.buttons === 0) return;
    seekTo(e.clientY);
  };

  const markBase = css({
    position: 'absolute',
    right: '0',
    height: '1px',
    cursor: 'pointer',
    transition: '[width 0.12s ease]',
    _hover: { width: '[13px]' },
    _focusVisible: { width: '[13px]' },
  });
  // 상태는 셋뿐이다. 색을 더 늘리면 눈금자가 범례를 요구하는 도표가 된다.
  const markTone: Record<RailMark['state'], string> = {
    unseen: css({ width: '[11px]', backgroundColor: 'text.subtle' }),
    seen: css({ width: '[7px]', backgroundColor: 'border.default' }),
    fail: css({ width: '[11px]', backgroundColor: 'accent.danger.default' }),
  };
</script>

<div
  bind:this={trackEl}
  class={css({ position: 'relative', width: '13px', flexShrink: '0', marginY: '48px', cursor: 'pointer', touchAction: 'none' })}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  role="presentation"
>
  <div class={css({ position: 'absolute', right: '0', insetY: '0', width: '1px', backgroundColor: 'border.subtle' })}></div>

  {#if viewport}
    <div
      style:top={`${viewport.start * 100}%`}
      style:height={`${Math.max(0.015, viewport.end - viewport.start) * 100}%`}
      class={css({ position: 'absolute', right: '0', width: '3px', backgroundColor: 'text.faint', pointerEvents: 'none' })}
    ></div>
  {/if}

  {#each marks as mark (mark.feedbackId)}
    <button
      style:top={`${mark.position * 100}%`}
      class={`${markBase} ${markTone[mark.state]}`}
      aria-label={`지적 ${mark.number}로 이동`}
      onclick={(e) => {
        e.stopPropagation();
        onSelect(mark.feedbackId);
      }}
      onpointerdown={(e) => e.stopPropagation()}
      type="button"
    ></button>
  {/each}
</div>
