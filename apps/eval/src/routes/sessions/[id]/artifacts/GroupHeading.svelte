<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { tooltip } from '@typie/ui/actions';
  import { labelGlossOf } from '$lib/feedback/glosses.ts';
  import type { LabelField } from '$lib/feedback/glosses.ts';

  // 그룹 제목 — 산출물의 상위 키를 원문 그대로 제목 서체(산세리프 semibold)로 세운다. 구조 이름은 데이터 토큰(mono)이 아니라
  // 제목이다. level 1=섹션 바로 아래(verdicts·tense…), level 2=그 안의 중첩 키(tense 아래의 시간 이탈 목록 등).
  // field를 주면 그 키의 한국어 풀이를 호버 툴팁으로 얹는다(glosses.ts).
  type Props = { label: string; level?: 1 | 2; field?: LabelField };
  const { label, level = 1, field }: Props = $props();

  const gloss = $derived(field === undefined ? null : labelGlossOf(field));
</script>

{#if level === 1}
  <h4
    class={css(
      { fontSize: '15px', fontWeight: 'semibold', letterSpacing: '[-0.01em]', color: 'text.default', width: 'fit' },
      gloss !== null && { cursor: 'help' },
    )}
    use:tooltip={{ message: gloss, placement: 'top', delay: 200 }}
  >
    {label}
  </h4>
{:else}
  <h5
    class={css(
      { marginTop: '6px', fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle', width: 'fit' },
      gloss !== null && { cursor: 'help' },
    )}
    use:tooltip={{ message: gloss, placement: 'top', delay: 200 }}
  >
    {label}
  </h5>
{/if}
