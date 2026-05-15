<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import BoldIcon from '~icons/lucide/bold';
  import ItalicIcon from '~icons/lucide/italic';
  import RedoIcon from '~icons/lucide/redo';
  import RemoveFormattingIcon from '~icons/lucide/remove-formatting';
  import StrikethroughIcon from '~icons/lucide/strikethrough';
  import UnderlineIcon from '~icons/lucide/underline';
  import UndoIcon from '~icons/lucide/undo';
  import { values } from '$lib/editor/values';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { graphql } from '$mearie';
  import ToolbarButton from './ToolbarButton.svelte';
  import type { LayoutMode, Message, Modifier, ModifierType, Tri } from '@typie/editor-ffi/browser';
  import type { BottomToolbar_document$key } from '$mearie';

  type Props = {
    document$key: BottomToolbar_document$key;
  };

  let { document$key }: Props = $props();

  const document = createFragment(
    graphql(`
      fragment BottomToolbar_document on Document {
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

  const MIXED = '__mixed__';

  type ToggleState = { active: boolean; indeterminate: boolean };

  const toggleState = (tri: Tri<undefined> | undefined): ToggleState => {
    if (tri?.type === 'uniform') return { active: true, indeterminate: false };
    if (tri?.type === 'mixed') return { active: false, indeterminate: true };
    return { active: false, indeterminate: false };
  };

  type SelectState =
    | { kind: 'placeholder'; selected: string }
    | { kind: 'mixed'; selected: string }
    | { kind: 'preset'; selected: string | number }
    | { kind: 'orphan'; selected: string | number; scalar: string | number };

  // `<select value>` matches options via Object.is against their typed value, with no
  // string coercion. Numeric `<option>` values must be compared as numbers, so `selected`
  // carries the raw scalar; only the string placeholder/mixed sentinels stay strings.
  const selectState = (tri: Tri<{ value: string | number }> | undefined, presets: readonly (string | number)[]): SelectState => {
    if (tri?.type !== 'uniform') {
      return tri?.type === 'mixed' ? { kind: 'mixed', selected: MIXED } : { kind: 'placeholder', selected: '' };
    }
    const scalar = tri.value.value;
    return presets.includes(scalar) ? { kind: 'preset', selected: scalar } : { kind: 'orphan', selected: scalar, scalar };
  };

  const boldS = $derived(toggleState(ctx.editor?.modifierState?.bold));
  const italicS = $derived(toggleState(ctx.editor?.modifierState?.italic));
  const strikethroughS = $derived(toggleState(ctx.editor?.modifierState?.strikethrough));
  const underlineS = $derived(toggleState(ctx.editor?.modifierState?.underline));

  const fontFamilyS = $derived(
    selectState(
      ctx.editor?.modifierState?.font_family,
      fontFamilies.map((f) => f.familyName),
    ),
  );
  const fontWeightS = $derived(
    selectState(
      ctx.editor?.modifierState?.font_weight,
      values.fontWeight.map((v) => v.value),
    ),
  );
  const fontSizeS = $derived(
    selectState(
      ctx.editor?.modifierState?.font_size,
      values.fontSize.map((v) => v.value),
    ),
  );
  const textColorS = $derived(
    selectState(
      ctx.editor?.modifierState?.text_color,
      values.textColor.map((v) => v.value),
    ),
  );
  const backgroundColorS = $derived(
    selectState(
      ctx.editor?.modifierState?.background_color,
      values.textBackgroundColor.map((v) => v.value),
    ),
  );
  const lineHeightS = $derived(
    selectState(
      ctx.editor?.modifierState?.line_height,
      values.lineHeight.map((v) => v.value),
    ),
  );
  const letterSpacingS = $derived(
    selectState(
      ctx.editor?.modifierState?.letter_spacing,
      values.letterSpacing.map((v) => v.value),
    ),
  );

  const enqueue = (message: Message) => {
    ctx.editor?.enqueue(message);
    ctx.editor?.focus();
  };

  const toggleModifier = (modifier_type: ModifierType) => {
    enqueue({ type: 'modifier', op: { type: 'toggle', modifier_type } });
  };

  const setModifier = (modifier: Modifier) => {
    enqueue({ type: 'modifier', op: { type: 'set', modifier } });
  };

  const setLayoutMode = (type: LayoutMode['type']) => {
    const layout_mode: LayoutMode =
      type === 'paginated'
        ? {
            type: 'paginated',
            page_width: 794,
            page_height: 1123,
            page_margin_top: 94,
            page_margin_bottom: 94,
            page_margin_left: 94,
            page_margin_right: 94,
          }
        : { type: 'continuous', max_width: 600 };
    enqueue({ type: 'node', op: { type: 'set_attrs', id: '0', attrs: { type: 'root', layout_mode } } });
  };

  const selectStyle = css.raw({
    fontSize: '12px',
    paddingX: '4px',
    paddingY: '2px',
    borderRadius: '4px',
    borderWidth: '1px',
    borderColor: 'border.subtle',
  });
</script>

<div
  class={css({
    display: 'flex',
    flexShrink: '0',
    alignItems: 'center',
    gap: '6px',
    paddingX: '12px',
    paddingY: '6px',
    borderBottomWidth: '1px',
    borderColor: 'border.subtle',
    backgroundColor: 'surface.default',
    flexWrap: 'wrap',
  })}
  role="toolbar"
>
  <ToolbarButton icon={UndoIcon} label="실행 취소" onclick={() => enqueue({ type: 'history', op: { type: 'undo' } })} />
  <ToolbarButton icon={RedoIcon} label="다시 실행" onclick={() => enqueue({ type: 'history', op: { type: 'redo' } })} />

  <div class={css({ width: '1px', height: '16px', backgroundColor: 'border.subtle' })}></div>

  <ToolbarButton
    active={boldS.active}
    icon={BoldIcon}
    indeterminate={boldS.indeterminate}
    label="굵게"
    onclick={() => toggleModifier('bold')}
  />
  <ToolbarButton
    active={italicS.active}
    icon={ItalicIcon}
    indeterminate={italicS.indeterminate}
    label="기울임"
    onclick={() => toggleModifier('italic')}
  />
  <ToolbarButton
    active={strikethroughS.active}
    icon={StrikethroughIcon}
    indeterminate={strikethroughS.indeterminate}
    label="취소선"
    onclick={() => toggleModifier('strikethrough')}
  />
  <ToolbarButton
    active={underlineS.active}
    icon={UnderlineIcon}
    indeterminate={underlineS.indeterminate}
    label="밑줄"
    onclick={() => toggleModifier('underline')}
  />

  <div class={css({ width: '1px', height: '16px', backgroundColor: 'border.subtle' })}></div>

  <select
    class={css(selectStyle)}
    onchange={(e) => setModifier({ type: 'font_family', value: e.currentTarget.value })}
    value={fontFamilyS.selected}
  >
    <option disabled value="">글꼴</option>
    {#if fontFamilyS.kind === 'mixed'}
      <option disabled value={MIXED}>(여러 값)</option>
    {:else if fontFamilyS.kind === 'orphan'}
      <option value={fontFamilyS.selected}>
        {document.data.selectableFontFamilies.find((f) => f.familyName === fontFamilyS.selected)?.displayName ?? fontFamilyS.selected}
      </option>
    {/if}
    {#each fontFamilies as f (f.id)}
      <option value={f.familyName}>{f.displayName}</option>
    {/each}
  </select>

  <select
    class={css(selectStyle)}
    onchange={(e) => setModifier({ type: 'font_weight', value: Number(e.currentTarget.value) })}
    value={fontWeightS.selected}
  >
    <option disabled value="">글꼴 굵기</option>
    {#if fontWeightS.kind === 'mixed'}
      <option disabled value={MIXED}>(여러 값)</option>
    {:else if fontWeightS.kind === 'orphan'}
      <option value={fontWeightS.selected}>{fontWeightS.scalar}</option>
    {/if}
    {#each values.fontWeight as { label, value } (value)}
      <option {value}>{label}</option>
    {/each}
  </select>

  <select
    class={css(selectStyle)}
    onchange={(e) => setModifier({ type: 'font_size', value: Number(e.currentTarget.value) })}
    value={fontSizeS.selected}
  >
    <option disabled value="">글꼴 크기</option>
    {#if fontSizeS.kind === 'mixed'}
      <option disabled value={MIXED}>(여러 값)</option>
    {:else if fontSizeS.kind === 'orphan'}
      <option value={fontSizeS.selected}>{Number(fontSizeS.scalar) / 100}</option>
    {/if}
    {#each values.fontSize as { label, value } (value)}
      <option {value}>{label}</option>
    {/each}
  </select>

  <div class={css({ width: '1px', height: '16px', backgroundColor: 'border.subtle' })}></div>

  <select
    class={css(selectStyle)}
    onchange={(e) => setModifier({ type: 'text_color', value: e.currentTarget.value })}
    value={textColorS.selected}
  >
    <option disabled value="">글씨 색</option>
    {#if textColorS.kind === 'mixed'}
      <option disabled value={MIXED}>(여러 값)</option>
    {:else if textColorS.kind === 'orphan'}
      <option value={textColorS.selected}>{textColorS.scalar}</option>
    {/if}
    {#each values.textColor as { label, value } (value)}
      <option {value}>{label}</option>
    {/each}
  </select>

  <select
    class={css(selectStyle)}
    onchange={(e) => setModifier({ type: 'background_color', value: e.currentTarget.value })}
    value={backgroundColorS.selected}
  >
    <option disabled value="">배경색</option>
    {#if backgroundColorS.kind === 'mixed'}
      <option disabled value={MIXED}>(여러 값)</option>
    {:else if backgroundColorS.kind === 'orphan'}
      <option value={backgroundColorS.selected}>{backgroundColorS.scalar}</option>
    {/if}
    {#each values.textBackgroundColor as { label, value } (value)}
      <option {value}>{label}</option>
    {/each}
  </select>

  <div class={css({ width: '1px', height: '16px', backgroundColor: 'border.subtle' })}></div>

  <select
    class={css(selectStyle)}
    onchange={(e) => setModifier({ type: 'line_height', value: Number(e.currentTarget.value) })}
    value={lineHeightS.selected}
  >
    <option disabled value="">행간</option>
    {#if lineHeightS.kind === 'mixed'}
      <option disabled value={MIXED}>(여러 값)</option>
    {:else if lineHeightS.kind === 'orphan'}
      <option value={lineHeightS.selected}>{lineHeightS.scalar}%</option>
    {/if}
    {#each values.lineHeight as { label, value } (value)}
      <option {value}>{label}</option>
    {/each}
  </select>

  <select
    class={css(selectStyle)}
    onchange={(e) => setModifier({ type: 'letter_spacing', value: Number(e.currentTarget.value) })}
    value={letterSpacingS.selected}
  >
    <option disabled value="">자간</option>
    {#if letterSpacingS.kind === 'mixed'}
      <option disabled value={MIXED}>(여러 값)</option>
    {:else if letterSpacingS.kind === 'orphan'}
      <option value={letterSpacingS.selected}>{letterSpacingS.scalar}%</option>
    {/if}
    {#each values.letterSpacing as { label, value } (value)}
      <option {value}>{label}</option>
    {/each}
  </select>

  <div class={css({ width: '1px', height: '16px', backgroundColor: 'border.subtle' })}></div>

  <ToolbarButton icon={RemoveFormattingIcon} label="서식 지우기" onclick={() => enqueue({ type: 'modifier', op: { type: 'clear_all' } })} />

  <div class={css({ width: '1px', height: '16px', backgroundColor: 'border.subtle' })}></div>

  <select
    class={css(selectStyle)}
    onchange={(e) => setLayoutMode(e.currentTarget.value as LayoutMode['type'])}
    value={ctx.editor?.rootAttrs?.layout_mode.type ?? 'continuous'}
  >
    <option value="paginated">페이지</option>
    <option value="continuous">연속</option>
  </select>
</div>
