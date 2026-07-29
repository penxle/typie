<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { autosize, tooltip } from '@typie/ui/actions';
  import type { FieldSpec } from '../../../core/contracts.ts';
  import type { AnalysisRender } from '../evaluations/fields.ts';

  type Props = {
    fields: FieldSpec[];
    value: Record<string, unknown>;
    onchange: (next: Record<string, unknown>) => void;
    readOnly?: boolean;
  };
  const { fields, value, onchange, readOnly = false }: Props = $props();

  const set = (key: string, next: unknown) => {
    if (readOnly) return;
    onchange({ ...value, [key]: next });
  };

  // 사유는 아니오를 고른 자리에만 열린다 — 늘 띄워두면 답해야 하는 문항으로 읽힌다.
  const rejected = $derived(fields.some((f) => (f.render as AnalysisRender).kind === 'yesNo' && value[f.key] === false));

  // 기본값은 '아직 안 고름'이다. 그래서 쉬고 있을 때 이 컨트롤은 조용하고,
  // 평가자가 고른 자리에만 색이 들어온다 — 화면에서 먼저 읽혀야 하는 것은 지적 본문이다.
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

  const scoreClass = (selected: boolean) =>
    css({
      paddingY: '6px',
      borderRadius: '6px',
      borderWidth: '1px',
      borderColor: selected ? 'border.strong' : 'border.default',
      backgroundColor: selected ? 'surface.dark' : 'surface.default',
      color: selected ? 'text.bright' : 'text.subtle',
      fontSize: '12px',
      fontWeight: selected ? 'bold' : 'normal',
      cursor: 'pointer',
      transition: '[background-color 0.15s ease, border-color 0.15s ease, color 0.15s ease]',
    });
</script>

<div class={flex({ direction: 'column', gap: '4px' })}>
  {#each fields as field (field.key)}
    {@const render = field.render as AnalysisRender}
    {#if render.kind === 'yesNo'}
      <div class={flex({ align: 'center', gap: '10px', minHeight: '26px' })}>
        <span
          class={css({
            flex: '1',
            fontSize: '12px',
            color: value[field.key] === undefined || value[field.key] === null ? 'text.subtle' : 'text.default',
            lineHeight: '[1.5]',
          })}
        >
          {render.question}
        </span>
        <div
          class={flex({
            flexShrink: '0',
            borderWidth: '1px',
            borderColor: value[field.key] === undefined || value[field.key] === null ? 'border.subtle' : 'border.default',
            borderRadius: '6px',
            // 채움이 테두리에 그대로 닿게 한다 — 사이에 1px이 남으면 그 틈이 먼저 눈에 들어온다.
            overflow: 'hidden',
            ['& > button + button']: { borderLeftWidth: '1px', borderColor: '[inherit]' },
          })}
        >
          <button
            class={optionClass(value[field.key] === true, 'neutral')}
            aria-pressed={value[field.key] === true}
            disabled={readOnly}
            onclick={() => set(field.key, true)}
            type="button"
          >
            예
          </button>
          <button
            class={optionClass(value[field.key] === false, 'danger')}
            aria-pressed={value[field.key] === false}
            disabled={readOnly}
            onclick={() => set(field.key, false)}
            type="button"
            use:tooltip={{ message: render.negative }}
          >
            아니오
          </button>
        </div>
      </div>
    {:else if render.kind === 'scale'}
      <div class={flex({ direction: 'column', gap: '6px', marginTop: '6px' })}>
        <span class={css({ fontSize: '13px', fontWeight: 'bold' })}>{render.question}</span>
        <div class={grid({ columns: 5, gap: '4px' })}>
          {#each render.anchors as anchor, i (anchor)}
            <button
              class={scoreClass(value[field.key] === i + 1)}
              disabled={readOnly}
              onclick={() => set(field.key, value[field.key] === i + 1 ? null : i + 1)}
              type="button"
            >
              {anchor}
            </button>
          {/each}
        </div>
      </div>
    {:else if render.kind === 'reason'}
      {#if rejected && !readOnly}
        <!-- 한 줄 입력은 길어지면 옆으로 숨는다 — 한 줄로 시작해 아래로 자라는 textarea를 쓴다. -->
        <textarea
          class={css({
            width: 'full',
            marginTop: '4px',
            borderBottomWidth: '1px',
            borderColor: 'border.default',
            paddingY: '4px',
            fontSize: '12px',
            backgroundColor: '[transparent]',
            resize: 'none',
            _focus: { borderColor: 'border.strong' },
          })}
          maxlength={render.maxLength}
          oninput={(e) => set(field.key, e.currentTarget.value)}
          placeholder={render.placeholder}
          rows="1"
          value={typeof value[field.key] === 'string' ? (value[field.key] as string) : ''}
          use:autosize={{ value: typeof value[field.key] === 'string' ? (value[field.key] as string) : '' }}></textarea>
      {/if}
    {:else}
      <!-- 길게 쓰라고 둔 칸이다 — 고정 높이 안 스크롤 대신 내용을 따라 아래로 자란다. -->
      <textarea
        class={css({
          width: 'full',
          marginTop: '10px',
          borderWidth: '1px',
          borderColor: 'border.default',
          borderRadius: '8px',
          padding: '8px',
          fontSize: '13px',
          minHeight: '44px',
          backgroundColor: 'surface.default',
          resize: 'none',
        })}
        disabled={readOnly}
        maxlength={render.maxLength}
        oninput={(e) => set(field.key, e.currentTarget.value)}
        placeholder={render.label}
        value={typeof value[field.key] === 'string' ? (value[field.key] as string) : ''}
        use:autosize={{ value: typeof value[field.key] === 'string' ? (value[field.key] as string) : '' }}></textarea>
    {/if}
  {/each}
</div>
