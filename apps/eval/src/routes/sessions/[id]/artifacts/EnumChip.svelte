<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { tooltip } from '@typie/ui/actions';
  import { glossOf } from '$lib/feedback/glosses.ts';
  import type { GlossField } from '$lib/feedback/glosses.ts';

  // enum 값 — 값 토큰은 필이다(id와 같은 모양). 키는 평문 mono라 모양으로 키와 값이 갈린다.
  // field를 주면 그 필드의 한국어 풀이를 호버 툴팁으로 얹는다(값 자체는 원문 그대로) — 풀이 사전은 glosses.ts.
  type Props = { value: string; field?: GlossField };
  const { value, field }: Props = $props();

  const gloss = $derived(field === undefined ? undefined : glossOf(field, value));
</script>

<span
  class={css(
    {
      display: 'inline-flex',
      alignItems: 'center',
      paddingX: '6px',
      paddingY: '1px',
      borderRadius: '4px',
      backgroundColor: 'surface.subtle',
      fontFamily: 'mono',
      fontSize: '11px',
      letterSpacing: '0',
      lineHeight: '[1.6]',
      color: 'text.subtle',
      whiteSpace: 'nowrap',
    },
    gloss !== undefined && { cursor: 'help' },
  )}
  use:tooltip={{ message: gloss ?? null, placement: 'top', delay: 200 }}
>
  {value}
</span>
