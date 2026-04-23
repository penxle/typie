<script lang="ts">
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
  import ToolbarButton from './ToolbarButton.svelte';
  import type { LayoutMode, Message, Modifier, ModifierType } from '@typie/editor-ffi/browser';

  const ctx = getEditorContext();

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
    enqueue({ type: 'doc', op: { type: 'set_attrs', attrs: { layout_mode } } });
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

  <ToolbarButton icon={BoldIcon} label="굵게" onclick={() => toggleModifier('bold')} />
  <ToolbarButton icon={ItalicIcon} label="기울임" onclick={() => toggleModifier('italic')} />
  <ToolbarButton icon={StrikethroughIcon} label="취소선" onclick={() => toggleModifier('strikethrough')} />
  <ToolbarButton icon={UnderlineIcon} label="밑줄" onclick={() => toggleModifier('underline')} />

  <div class={css({ width: '1px', height: '16px', backgroundColor: 'border.subtle' })}></div>

  <select class={css(selectStyle)} onchange={(e) => setModifier({ type: 'font_size', value: Number(e.currentTarget.value) })}>
    <option disabled selected value="">글꼴 크기</option>
    {#each values.fontSize as { label, value } (value)}
      <option {value}>{label}</option>
    {/each}
  </select>

  <select class={css(selectStyle)} onchange={(e) => setModifier({ type: 'font_weight', value: Number(e.currentTarget.value) })}>
    <option disabled selected value="">글꼴 굵기</option>
    {#each values.fontWeight as { label, value } (value)}
      <option {value}>{label}</option>
    {/each}
  </select>

  <div class={css({ width: '1px', height: '16px', backgroundColor: 'border.subtle' })}></div>

  <select class={css(selectStyle)} onchange={(e) => setModifier({ type: 'text_color', value: e.currentTarget.value })}>
    <option disabled selected value="">글씨 색</option>
    {#each values.textColor as { label, value } (value)}
      <option {value}>{label}</option>
    {/each}
  </select>

  <select class={css(selectStyle)} onchange={(e) => setModifier({ type: 'background_color', value: e.currentTarget.value })}>
    <option disabled selected value="">배경색</option>
    {#each values.textBackgroundColor as { label, value } (value)}
      <option {value}>{label}</option>
    {/each}
  </select>

  <div class={css({ width: '1px', height: '16px', backgroundColor: 'border.subtle' })}></div>

  <select class={css(selectStyle)} onchange={(e) => setModifier({ type: 'line_height', value: Number(e.currentTarget.value) })}>
    <option disabled selected value="">행간</option>
    {#each values.lineHeight as { label, value } (value)}
      <option {value}>{label}</option>
    {/each}
  </select>

  <select class={css(selectStyle)} onchange={(e) => setModifier({ type: 'letter_spacing', value: Number(e.currentTarget.value) })}>
    <option disabled selected value="">자간</option>
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
    value={ctx.editor?.documentAttrs?.layout_mode.type ?? 'continuous'}
  >
    <option value="paginated">페이지</option>
    <option value="continuous">연속</option>
  </select>
</div>
