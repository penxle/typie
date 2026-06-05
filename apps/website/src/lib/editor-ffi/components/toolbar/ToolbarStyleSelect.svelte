<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, SearchableDropdown } from '@typie/ui/components';
  import RemoveFormattingIcon from '~icons/lucide/remove-formatting';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';

  const ctx = getEditorContext();

  const styleEntries = $derived(ctx.editor?.styleEntries ?? []);

  const currentStyleId = $derived(ctx.editor?.appliedStyle?.type === 'uniform' ? ctx.editor.appliedStyle.value.value : undefined);

  const targetBlockIds = $derived.by(() => {
    const editor = ctx.editor;
    if (!editor) return [];
    const blockState = editor.blockState;
    if (!blockState) return [];
    const isCollapsed = editor.isSelectionCollapsed ?? true;
    if (isCollapsed) {
      const head = blockState.ancestors[0];
      return head ? [head.id] : [];
    }
    return blockState.nodes.map((n) => n.id);
  });

  const clearStyles = () => {
    const editor = ctx.editor;
    if (!editor || currentStyleId === undefined) return;
    editor.enqueue({ type: 'style', op: { type: 'unset_in_selection' } });
    editor.focus();
  };
</script>

{#snippet clearStyleItem()}
  <div class={flex({ alignItems: 'center', gap: '4px' })}>
    <Icon
      style={css.raw({ color: 'text.faint', transitionProperty: '[none]', _groupHover: { color: 'text.brand' } })}
      icon={RemoveFormattingIcon}
      size={14}
    />
    <span class={css({ color: 'text.subtle', _groupHover: { color: 'text.brand' } })}>스타일 해제</span>
  </div>
{/snippet}

<SearchableDropdown
  style={css.raw({ width: '120px' })}
  disabled={targetBlockIds.length === 0}
  extraItems={currentStyleId === undefined
    ? undefined
    : [
        {
          onclick: clearStyles,
          content: clearStyleItem,
        },
      ]}
  getLabel={(value) => styleEntries.find((s) => s.id === value)?.name ?? '(알 수 없는 스타일)'}
  items={styleEntries.map((s) => ({ value: s.id, label: s.name }))}
  label="문단 스타일"
  onEscape={() => ctx.editor?.focus()}
  onchange={(styleId, options) => {
    const editor = ctx.editor;
    if (!editor) return;
    editor.enqueue({ type: 'style', op: { type: 'apply_to_selection', style_id: styleId } });
    if (options?.shouldFocus) {
      editor.focus();
    }
  }}
  placeholder="-"
  value={currentStyleId}
/>
