<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import type { Verdict } from '$lib/domain/verdicts.ts';

  // 기본값은 '아직 안 고름'이다. 그래서 쉬고 있을 때 이 컨트롤은 조용하고,
  // 평가자가 고른 자리에만 색이 들어온다 — 화면에서 먼저 읽혀야 하는 것은 피드백 본문이다.
  type Props = { question: string; negative: string; value: Verdict; onChange: (value: Verdict) => void };
  const { question, negative, value, onChange }: Props = $props();

  const optionClass = (selected: boolean, tone: 'neutral' | 'danger') =>
    css({
      paddingX: '9px',
      paddingY: '3px',
      backgroundColor: selected ? (tone === 'danger' ? 'accent.danger.subtle' : 'surface.muted') : '[transparent]',
      color: selected ? (tone === 'danger' ? 'text.danger' : 'text.default') : 'text.faint',
      fontSize: '12px',
      fontWeight: selected ? 'bold' : 'normal',
      cursor: 'pointer',
      transition: '[background-color 0.12s ease, color 0.12s ease]',
      ['&:hover:not([aria-pressed="true"])']: { color: 'text.subtle' },
    });
</script>

<div class={flex({ align: 'center', gap: '10px', minHeight: '26px' })}>
  <span class={css({ flex: '1', fontSize: '12px', color: value === null ? 'text.subtle' : 'text.default', lineHeight: '[1.5]' })}>
    {question}
  </span>
  <div
    class={flex({
      flexShrink: '0',
      borderWidth: '1px',
      borderColor: value === null ? 'border.subtle' : 'border.default',
      borderRadius: '6px',
      // 채움이 테두리에 그대로 닿게 한다 — 사이에 1px이 남으면 그 틈이 먼저 눈에 들어온다.
      overflow: 'hidden',
      ['& > button + button']: { borderLeftWidth: '1px', borderColor: '[inherit]' },
    })}
  >
    <button class={optionClass(value === true, 'neutral')} aria-pressed={value === true} onclick={() => onChange(true)} type="button">
      예
    </button>
    <button
      class={optionClass(value === false, 'danger')}
      aria-pressed={value === false}
      onclick={() => onChange(false)}
      type="button"
      use:tooltip={{ message: negative }}
    >
      아니오
    </button>
  </div>
</div>
