<script lang="ts">
  import { getAppContext } from '@typie/ui/context';
  import LetterSpacingIcon from '~icons/typie/letter-spacing';
  import LineHeightIcon from '~icons/typie/line-height';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { values } from '$lib/editor-ffi/values';
  import { toolbarRecent } from './toolbar-recent.svelte';
  import ToolbarIcon from './ToolbarIcon.svelte';
  import ToolbarPanel from './ToolbarPanel.svelte';
  import ToolbarPanelDropdown from './ToolbarPanelDropdown.svelte';
  import type { ToolbarPanelItem } from './ToolbarPanel.svelte';

  type Props = {
    kind: 'align' | 'lineHeight' | 'letterSpacing';
    disabled?: boolean;
  };

  let { kind, disabled = false }: Props = $props();

  const app = getAppContext();
  const ctx = getEditorContext();
  const recent = toolbarRecent(app.userId);

  const alignKeywords: Record<string, string[]> = {
    left: ['left'],
    center: ['center'],
    right: ['right'],
    justify: ['justify'],
  };

  const currentAlign = $derived(
    ctx.editor?.modifierState?.alignment?.type === 'uniform' ? ctx.editor.modifierState.alignment.value.value : undefined,
  );
  const currentLineHeight = $derived(
    ctx.editor?.modifierState?.line_height?.type === 'uniform' ? ctx.editor.modifierState.line_height.value.value : undefined,
  );
  const currentLetterSpacing = $derived(
    ctx.editor?.modifierState?.letter_spacing?.type === 'uniform' ? ctx.editor.modifierState.letter_spacing.value.value : undefined,
  );

  const label = $derived(kind === 'align' ? '문단 정렬' : kind === 'lineHeight' ? '문단 행간' : '자간');

  const items = $derived<ToolbarPanelItem[]>(
    kind === 'align'
      ? values.textAlign.map(({ label: text, value, icon }) => ({
          id: value,
          label: text,
          chipLabel: text.replace(/\s*정렬$/, ''),
          icon,
          keywords: alignKeywords[value],
          selected: value === currentAlign,
        }))
      : kind === 'lineHeight'
        ? values.lineHeight.map(({ label: text, value }) => ({ id: String(value), label: text, selected: value === currentLineHeight }))
        : values.letterSpacing.map(({ label: text, value }) => ({
            id: String(value),
            label: text,
            selected: value === currentLetterSpacing,
          })),
  );

  const anchorIcon = $derived(
    kind === 'align'
      ? (values.textAlign.find((a) => a.value === currentAlign)?.icon ?? values.textAlign[0].icon)
      : kind === 'lineHeight'
        ? LineHeightIcon
        : LetterSpacingIcon,
  );

  const apply = (id: string, close: () => void) => {
    if (kind === 'align') {
      const value = values.textAlign.find((a) => a.value === id)?.value;
      if (value) ctx.editor?.enqueue({ type: 'modifier', op: { type: 'set', modifier: { type: 'alignment', value } } });
    } else if (kind === 'lineHeight') {
      ctx.editor?.enqueue({ type: 'modifier', op: { type: 'set', modifier: { type: 'line_height', value: Number(id) } } });
    } else {
      ctx.editor?.enqueue({ type: 'modifier', op: { type: 'set', modifier: { type: 'letter_spacing', value: Number(id) } } });
    }
    recent.remember(kind, id);
    close();
    ctx.editor?.focus();
  };
</script>

<ToolbarPanelDropdown {disabled} {label}>
  {#snippet anchor()}
    <ToolbarIcon icon={anchorIcon} />
  {/snippet}

  {#snippet panel({ close })}
    <ToolbarPanel {items} onselect={(id) => apply(id, close)} placeholder={label} recentIds={recent.ids(kind)} />
  {/snippet}
</ToolbarPanelDropdown>
