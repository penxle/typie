<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { tooltip } from '@typie/ui/actions';
  import { labelGlossOf } from '$lib/feedback/glosses.ts';
  import type { Snippet } from 'svelte';
  import type { LabelField } from '$lib/feedback/glosses.ts';

  // 필드 키는 산출물의 원문 키 그대로다(번역 없음) — 평문 mono. row=키 96px 왼쪽·값 오른쪽의 정의 목록형(기본),
  // block=키 위·값 아래(값이 표·목록처럼 폭을 쓰는 경우). field를 주면 그 키의 한국어 풀이를 호버 툴팁으로 얹는다.
  type Props = { label: string; layout?: 'row' | 'block'; field?: LabelField; children: Snippet };
  const { label, layout = 'row', field, children }: Props = $props();

  const gloss = $derived(field === undefined ? null : labelGlossOf(field));
</script>

<div
  class={css(
    layout === 'row'
      ? { display: 'grid', gridTemplateColumns: '[96px minmax(0, 1fr)]', columnGap: '14px', alignItems: 'baseline' }
      : { display: 'flex', flexDirection: 'column', gap: '8px' },
  )}
>
  <span
    class={css(
      {
        fontFamily: 'mono',
        fontSize: '11px',
        letterSpacing: '0',
        lineHeight: '[1.6]',
        color: 'text.faint',
        wordBreak: 'normal',
        overflowWrap: 'anywhere',
        width: 'fit',
        maxWidth: 'full',
      },
      gloss !== null && { cursor: 'help' },
    )}
    use:tooltip={{ message: gloss, placement: 'top', delay: 200 }}
  >
    {label}
  </span>
  <div
    class={css({
      minWidth: '0',
      fontSize: '14px',
      lineHeight: '[1.7]',
      color: 'text.default',
      wordBreak: 'keep-all',
      overflowWrap: 'anywhere',
    })}
  >
    {@render children()}
  </div>
</div>
