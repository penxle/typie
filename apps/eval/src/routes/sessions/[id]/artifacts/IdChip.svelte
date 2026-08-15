<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { jumpTo } from './jump.ts';

  // 식별자 — 값 토큰의 필(enum과 같은 모양). target이 있으면 모달 안 그 자리로 가는 링크(점선 밑줄·눌림), 없으면 정적 표시.
  // 대상 실재는 호출부(linkTo)가 판정한다. quiet는 항목 첫 줄의 자기 id — 참조되는 자리가 아니라 꼬리표라 필 없이
  // 연한 평문으로 눕힌다(중요 정보가 아닌데 필이 눈을 먼저 끌었다).
  type Props = { value: string; target?: string; quiet?: boolean };
  const { value, target, quiet = false }: Props = $props();

  const base = css.raw({
    display: 'inline-flex',
    alignItems: 'center',
    maxWidth: 'full',
    paddingX: '6px',
    paddingY: '1px',
    borderRadius: '4px',
    backgroundColor: 'surface.subtle',
    fontFamily: 'mono',
    fontSize: '11px',
    letterSpacing: '0',
    lineHeight: '[1.6]',
    color: 'text.subtle',
    // 긴 kebab id는 하이픈에서만 줄이 바뀐다 — break-word는 min-content 폭을 좁히지 않아 표 열도 단어 중간을 자르지 않는다.
    wordBreak: 'normal',
    overflowWrap: 'break-word',
  });
</script>

{#if quiet}
  <span
    class={css({
      fontFamily: 'mono',
      fontSize: '11px',
      letterSpacing: '0',
      lineHeight: '[1.6]',
      color: 'text.faint',
      wordBreak: 'normal',
      overflowWrap: 'break-word',
    })}
  >
    {value}
  </span>
{:else if target}
  <button
    class={css(base, {
      cursor: 'pointer',
      textDecoration: 'underline',
      textDecorationStyle: 'dotted',
      textDecorationColor: 'text.faint',
      textUnderlineOffset: '3px',
      transition: 'common',
      transitionDuration: '[160ms]',
      transitionTimingFunction: '[cubic-bezier(0.23, 1, 0.32, 1)]',
      _hover: { color: 'text.brand', backgroundColor: 'accent.brand.subtle', textDecorationColor: 'text.brand' },
      _active: { transform: '[scale(0.97)]' },
    })}
    onclick={() => jumpTo(target)}
    type="button"
  >
    {value}
  </button>
{:else}
  <span class={css(base)}>{value}</span>
{/if}
