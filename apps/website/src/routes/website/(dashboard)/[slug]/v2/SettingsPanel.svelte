<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { values } from '$lib/editor/values';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { graphql } from '$mearie';
  import { defaultContinuousLayout, defaultPaginatedLayout, setRootLayoutMode, setRootModifier } from './root-attrs';
  import type { LayoutMode, Modifier, ModifierType } from '@typie/editor-ffi/browser';
  import type { SettingsPanel_document$key } from '$mearie';

  type Props = {
    open: boolean;
    onClose: () => void;
    document$key: SettingsPanel_document$key;
  };

  let { open, onClose, document$key }: Props = $props();

  const document = createFragment(
    graphql(`
      fragment SettingsPanel_document on Document {
        id
        selectableFontFamilies: fontFamilies(sources: [DEFAULT, USER]) {
          id
          familyName
          displayName
          state
        }
      }
    `),
    () => document$key,
  );

  const fontFamilies = $derived(document.data.selectableFontFamilies.filter((f) => f.state === 'ACTIVE'));

  const ctx = getEditorContext();

  const mod = <T extends ModifierType>(type: T) =>
    ctx.editor?.rootModifiers?.find((m): m is Extract<Modifier, { type: T }> => m.type === type);

  const layoutMode = $derived(ctx.editor?.rootAttrs?.layout_mode);

  const setMod = (modifier: Modifier) => {
    setRootModifier(ctx.editor, modifier);
    ctx.editor?.focus();
  };

  const setLayout = (layout_mode: LayoutMode) => {
    setRootLayoutMode(ctx.editor, layout_mode);
    ctx.editor?.focus();
  };

  const patchPaginated = (patch: Partial<Extract<LayoutMode, { type: 'paginated' }>>) => {
    if (layoutMode?.type !== 'paginated') return;
    setLayout({ ...layoutMode, ...patch });
  };

  const selectStyle = css.raw({
    fontSize: '12px',
    paddingX: '4px',
    paddingY: '2px',
    borderRadius: '4px',
    borderWidth: '1px',
    borderColor: 'border.subtle',
  });

  const rowStyle = css.raw({
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    gap: '12px',
    paddingX: '16px',
    paddingY: '6px',
    fontSize: '12px',
  });

  const numberInputStyle = css.raw({
    width: '72px',
    fontSize: '12px',
    paddingX: '4px',
    paddingY: '2px',
    borderRadius: '4px',
    borderWidth: '1px',
    borderColor: 'border.subtle',
    textAlign: 'right',
  });
</script>

<aside
  class={css({
    display: open ? 'flex' : 'none',
    flexDirection: 'column',
    width: '280px',
    flexShrink: '0',
    borderLeftWidth: '1px',
    borderColor: 'border.subtle',
    backgroundColor: 'surface.default',
    overflowY: 'auto',
  })}
>
  <div
    class={css({
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      paddingX: '16px',
      paddingY: '10px',
      borderBottomWidth: '1px',
      borderColor: 'border.subtle',
      fontSize: '12px',
      fontWeight: 'semibold',
    })}
  >
    문서 설정
    <button class={css({ cursor: 'pointer', color: 'text.muted' })} onclick={onClose} type="button">✕</button>
  </div>

  <div class={css({ paddingX: '16px', paddingTop: '12px', fontSize: '11px', fontWeight: 'semibold', color: 'text.muted' })}>
    기본 스타일
  </div>

  <div class={css(rowStyle)}>
    <span>폰트 패밀리</span>
    <select
      class={css(selectStyle)}
      onchange={(e) => setMod({ type: 'font_family', value: e.currentTarget.value })}
      value={mod('font_family')?.value ?? ''}
    >
      {#each fontFamilies as f (f.id)}
        <option value={f.familyName}>{f.displayName}</option>
      {/each}
    </select>
  </div>

  <div class={css(rowStyle)}>
    <span>폰트 크기</span>
    <select
      class={css(selectStyle)}
      onchange={(e) => setMod({ type: 'font_size', value: Number(e.currentTarget.value) })}
      value={mod('font_size')?.value ?? ''}
    >
      {#each values.fontSize as { label, value } (value)}
        <option {value}>{label}</option>
      {/each}
    </select>
  </div>

  <div class={css(rowStyle)}>
    <span>폰트 굵기</span>
    <select
      class={css(selectStyle)}
      onchange={(e) => setMod({ type: 'font_weight', value: Number(e.currentTarget.value) })}
      value={mod('font_weight')?.value ?? ''}
    >
      {#each values.fontWeight as { label, value } (value)}
        <option {value}>{label}</option>
      {/each}
    </select>
  </div>

  <div class={css(rowStyle)}>
    <span>글자 색</span>
    <select
      class={css(selectStyle)}
      onchange={(e) => setMod({ type: 'text_color', value: e.currentTarget.value })}
      value={mod('text_color')?.value ?? ''}
    >
      {#each values.textColor as { label, value } (value)}
        <option {value}>{label}</option>
      {/each}
    </select>
  </div>

  <div class={css(rowStyle)}>
    <span>배경 색</span>
    <select
      class={css(selectStyle)}
      onchange={(e) => setMod({ type: 'background_color', value: e.currentTarget.value })}
      value={mod('background_color')?.value ?? ''}
    >
      {#each values.textBackgroundColor as { label, value } (value)}
        <option {value}>{label}</option>
      {/each}
    </select>
  </div>

  <div class={css(rowStyle)}>
    <span>자간</span>
    <select
      class={css(selectStyle)}
      onchange={(e) => setMod({ type: 'letter_spacing', value: Number(e.currentTarget.value) })}
      value={mod('letter_spacing')?.value ?? ''}
    >
      {#each values.letterSpacing as { label, value } (value)}
        <option {value}>{label}</option>
      {/each}
    </select>
  </div>

  <div class={css(rowStyle)}>
    <span>행간</span>
    <select
      class={css(selectStyle)}
      onchange={(e) => setMod({ type: 'line_height', value: Number(e.currentTarget.value) })}
      value={mod('line_height')?.value ?? ''}
    >
      {#each values.lineHeight as { label, value } (value)}
        <option {value}>{label}</option>
      {/each}
    </select>
  </div>

  <div class={css({ paddingX: '16px', paddingTop: '12px', fontSize: '11px', fontWeight: 'semibold', color: 'text.muted' })}>
    세부 레이아웃
  </div>

  <div class={css(rowStyle)}>
    <span>첫 줄 들여쓰기</span>
    <select
      class={css(selectStyle)}
      onchange={(e) => setMod({ type: 'paragraph_indent', value: Number(e.currentTarget.value) })}
      value={mod('paragraph_indent')?.value ?? 0}
    >
      {#each values.paragraphIndent as { label, value } (value)}
        <option {value}>{label}</option>
      {/each}
    </select>
  </div>

  <div class={css(rowStyle)}>
    <span>문단 사이 간격</span>
    <select
      class={css(selectStyle)}
      onchange={(e) => setMod({ type: 'block_gap', value: Number(e.currentTarget.value) })}
      value={mod('block_gap')?.value ?? 0}
    >
      {#each values.blockGap as { label, value } (value)}
        <option {value}>{label}</option>
      {/each}
    </select>
  </div>

  <div class={css({ paddingX: '16px', paddingTop: '12px', fontSize: '11px', fontWeight: 'semibold', color: 'text.muted' })}>레이아웃</div>

  <div class={css(rowStyle)}>
    <span>모드</span>
    <select
      class={css(selectStyle)}
      onchange={(e) =>
        setLayout((e.currentTarget.value as LayoutMode['type']) === 'paginated' ? defaultPaginatedLayout() : defaultContinuousLayout())}
      value={layoutMode?.type ?? 'continuous'}
    >
      <option value="continuous">연속</option>
      <option value="paginated">페이지</option>
    </select>
  </div>

  {#if layoutMode?.type === 'continuous'}
    <div class={css(rowStyle)}>
      <span>본문 폭</span>
      <select
        class={css(selectStyle)}
        onchange={(e) => setLayout({ type: 'continuous', max_width: Number(e.currentTarget.value) })}
        value={layoutMode.max_width}
      >
        {#each values.maxWidth as { label, value } (value)}
          <option {value}>{label}</option>
        {/each}
      </select>
    </div>
  {:else if layoutMode?.type === 'paginated'}
    <div class={css(rowStyle)}>
      <span>너비</span>
      <input
        class={css(numberInputStyle)}
        onchange={(e) => patchPaginated({ page_width: Math.max(100, Number(e.currentTarget.value)) })}
        type="number"
        value={layoutMode.page_width}
      />
    </div>
    <div class={css(rowStyle)}>
      <span>높이</span>
      <input
        class={css(numberInputStyle)}
        onchange={(e) => patchPaginated({ page_height: Math.max(100, Number(e.currentTarget.value)) })}
        type="number"
        value={layoutMode.page_height}
      />
    </div>
    <div class={css(rowStyle)}>
      <span>여백 상/하</span>
      <span class={css({ display: 'flex', gap: '4px' })}>
        <input
          class={css(numberInputStyle)}
          onchange={(e) => patchPaginated({ page_margin_top: Math.max(0, Number(e.currentTarget.value)) })}
          type="number"
          value={layoutMode.page_margin_top}
        />
        <input
          class={css(numberInputStyle)}
          onchange={(e) => patchPaginated({ page_margin_bottom: Math.max(0, Number(e.currentTarget.value)) })}
          type="number"
          value={layoutMode.page_margin_bottom}
        />
      </span>
    </div>
    <div class={css(rowStyle)}>
      <span>여백 좌/우</span>
      <span class={css({ display: 'flex', gap: '4px' })}>
        <input
          class={css(numberInputStyle)}
          onchange={(e) => patchPaginated({ page_margin_left: Math.max(0, Number(e.currentTarget.value)) })}
          type="number"
          value={layoutMode.page_margin_left}
        />
        <input
          class={css(numberInputStyle)}
          onchange={(e) => patchPaginated({ page_margin_right: Math.max(0, Number(e.currentTarget.value)) })}
          type="number"
          value={layoutMode.page_margin_right}
        />
      </span>
    </div>
  {/if}

  <div class={css({ height: '16px' })}></div>
</aside>
