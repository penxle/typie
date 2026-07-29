<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { autosize, tooltip } from '@typie/ui/actions';
  import { reasonKinds } from '../evaluations/fields.ts';
  import type { FieldSpec } from '../../../core/contracts.ts';
  import type { EditorialRender } from '../evaluations/fields.ts';

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
  const rejected = $derived(
    fields.some((f) => {
      const kind = (f.render as EditorialRender).kind;
      return (kind === 'yesNo' || kind === 'triState') && value[f.key] === false;
    }),
  );

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

  const kindChipClass = (selected: boolean) =>
    css({
      paddingX: '8px',
      paddingY: '2px',
      borderWidth: '1px',
      borderColor: selected ? 'surface.dark' : 'border.default',
      borderRadius: 'full',
      fontSize: '11px',
      backgroundColor: selected ? 'surface.dark' : '[transparent]',
      color: selected ? 'text.bright' : 'text.faint',
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

<!-- 잠긴 그룹은 눈에 보이게 흐린다 — 버튼 disabled만으로는 활성처럼 읽혀서, 확정된 단계를
     다시 고치려는 시도를 부른다. -->
<div
  style:opacity={readOnly ? 0.45 : 1}
  style:pointer-events={readOnly ? 'none' : 'auto'}
  class={flex({ direction: 'column', gap: '4px' })}
>
  {#each fields as field (field.key)}
    {@const render = field.render as EditorialRender}
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
    {:else if render.kind === 'triState'}
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
          <button
            class={optionClass(value[field.key] === 'unknown', 'neutral')}
            aria-pressed={value[field.key] === 'unknown'}
            disabled={readOnly}
            onclick={() => set(field.key, 'unknown')}
            type="button"
          >
            {render.unknownLabel}
          </button>
        </div>
      </div>
    {:else if render.kind === 'choice'}
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
            overflow: 'hidden',
            ['& > button + button']: { borderLeftWidth: '1px', borderColor: '[inherit]' },
          })}
        >
          {#each render.options as option (option.value)}
            <button
              class={optionClass(value[field.key] === option.value, 'neutral')}
              aria-pressed={value[field.key] === option.value}
              disabled={readOnly}
              onclick={() => set(field.key, option.value)}
              type="button"
            >
              {option.label}
            </button>
          {/each}
        </div>
      </div>
    {:else if render.kind === 'reasonKind'}
      {@const kinds = reasonKinds(value[field.key])}
      {#if readOnly}
        <!-- 동결본에서는 고른 분류만 남긴다 — 입력 UI를 그대로 두면 답할 수 있는 것처럼 읽힌다. -->
        {@const picked = render.options.filter((o) => kinds.includes(o.value))}
        {#if picked.length > 0}
          <div class={flex({ align: 'center', gap: '4px', flexWrap: 'wrap', marginTop: '4px' })}>
            {#each picked as option (option.value)}
              <span class={kindChipClass(true)}>{option.label}</span>
            {/each}
          </div>
        {/if}
      {:else if rejected}
        <div class={flex({ align: 'center', gap: '4px', flexWrap: 'wrap', marginTop: '4px' })}>
          {#each render.options as option (option.value)}
            {@const selected = kinds.includes(option.value)}
            <button
              class={kindChipClass(selected)}
              aria-pressed={selected}
              onclick={() => {
                const next = selected ? kinds.filter((v) => v !== option.value) : [...kinds, option.value];
                set(field.key, next.length > 0 ? next : null);
              }}
              type="button"
            >
              {option.label}
            </button>
          {/each}
        </div>
      {/if}
    {:else if render.kind === 'scale'}
      <!-- 질문 타이포그래피는 위젯 종류와 무관하게 같아야 한다 — 한 문항만 굵으면 위계가 생긴 것처럼 읽힌다. -->
      <div class={flex({ direction: 'column', gap: '6px' })}>
        <span
          class={css({
            fontSize: '12px',
            color: value[field.key] === undefined || value[field.key] === null ? 'text.subtle' : 'text.default',
            lineHeight: '[1.5]',
          })}
        >
          {render.question}
        </span>
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
      {#if readOnly}
        <!-- 동결본에서는 써 둔 사유를 텍스트로 남긴다 — 숨기면 답한 내용이 사라진 것처럼 보인다. -->
        {#if typeof value[field.key] === 'string' && value[field.key]}
          <p class={css({ marginTop: '2px', fontSize: '12px', color: 'text.subtle', whiteSpace: 'pre-wrap' })}>{value[field.key]}</p>
        {/if}
      {:else if render.forKey ? value[render.forKey] === false : rejected}
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
      <!-- 라벨을 placeholder로만 주면 입력하는 순간 이 칸이 무엇이었는지 사라진다 — 위에 상시로 둔다. -->
      {#if readOnly}
        <!-- 동결본에서는 내용 있는 칸만 텍스트로 남긴다 — 빈 입력 상자는 소음이다. -->
        {#if typeof value[field.key] === 'string' && value[field.key]}
          <div class={css({ marginTop: '8px' })}>
            <span class={css({ display: 'block', marginBottom: '3px', fontSize: '12px', color: 'text.subtle' })}>{render.label}</span>
            <p class={css({ fontSize: '13px', lineHeight: '[1.6]', whiteSpace: 'pre-wrap' })}>{value[field.key]}</p>
          </div>
        {/if}
      {:else}
        <div class={css({ marginTop: '8px' })}>
          <span class={css({ display: 'block', marginBottom: '3px', fontSize: '12px', color: 'text.subtle' })}>{render.label}</span>
          <!-- 길게 쓰라고 둔 칸이다 — 고정 높이 안 스크롤 대신 내용을 따라 아래로 자란다. -->
          <textarea
            class={css({
              width: 'full',
              borderWidth: '1px',
              borderColor: 'border.default',
              borderRadius: '8px',
              padding: '8px',
              fontSize: '13px',
              minHeight: '44px',
              backgroundColor: 'surface.default',
              resize: 'none',
            })}
            maxlength={render.maxLength}
            oninput={(e) => set(field.key, e.currentTarget.value)}
            value={typeof value[field.key] === 'string' ? (value[field.key] as string) : ''}
            use:autosize={{ value: typeof value[field.key] === 'string' ? (value[field.key] as string) : '' }}></textarea>
        </div>
      {/if}
    {/if}
  {/each}
</div>
