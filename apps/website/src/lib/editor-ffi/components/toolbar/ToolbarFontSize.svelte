<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getAppContext } from '@typie/ui/context';
  import { clamp } from '@typie/ui/utils';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { values } from '$lib/editor-ffi/values';
  import { toolbarRecent } from './toolbar-recent.svelte';
  import ToolbarPanel from './ToolbarPanel.svelte';
  import ToolbarPanelDropdown from './ToolbarPanelDropdown.svelte';
  import type { ToolbarPanelItem } from './ToolbarPanel.svelte';

  type Props = {
    disabled?: boolean;
  };

  let { disabled = false }: Props = $props();

  const app = getAppContext();
  const ctx = getEditorContext();
  const recent = toolbarRecent(app.userId);

  const currentFontSize = $derived(
    ctx.editor?.modifierState?.font_size?.type === 'uniform' ? ctx.editor.modifierState.font_size.value.value : undefined,
  );

  const recentIds = $derived(recent.ids('fontSize'));

  const sizes = $derived.by(() => {
    const merged: number[] = values.fontSize.map(({ value }) => value);
    for (const id of recentIds) {
      const value = Number(id);
      if (Number.isFinite(value) && !merged.includes(value)) merged.push(value);
    }
    if (currentFontSize !== undefined && !merged.includes(currentFontSize)) merged.push(currentFontSize);
    return merged.toSorted((left, right) => left - right);
  });

  const items = $derived<ToolbarPanelItem[]>(
    sizes.map((value) => ({
      id: String(value),
      label: `${value / 100}pt`,
      keywords: [String(value / 100)],
      selected: value === currentFontSize,
    })),
  );

  const parse = (query: string) => {
    const match = /^(\d{1,3}(?:\.\d+)?)\s*(?:pt)?$/i.exec(query);
    if (!match) return null;
    const value = Math.round(Number.parseFloat(match[1]) * 100);
    return value >= values.minFontSize && value <= values.maxFontSize ? value : null;
  };

  const apply = (value: number, close: () => void) => {
    const clamped = clamp(Math.round(value), values.minFontSize, values.maxFontSize);
    ctx.editor?.enqueue({ type: 'modifier', op: { type: 'set', modifier: { type: 'font_size', value: clamped } } });
    recent.remember('fontSize', String(clamped));
    close();
    ctx.editor?.focus();
  };
</script>

<ToolbarPanelDropdown style={css.raw({ maxWidth: '180px' })} chevron {disabled} label="폰트 크기">
  {#snippet anchor()}
    <span
      class={css({
        paddingLeft: '2px',
        fontSize: '13px',
        fontWeight: 'medium',
        fontVariantNumeric: 'tabular-nums',
        whiteSpace: 'nowrap',
        truncate: true,
      })}
    >
      {currentFontSize === undefined ? '-' : `${currentFontSize / 100}pt`}
    </span>
  {/snippet}

  {#snippet panel({ close })}
    <ToolbarPanel
      free={{
        accepts: (query) => parse(query) !== null,
        apply: (query) => {
          const value = parse(query);
          if (value !== null) apply(value, close);
        },
        display: (query) => `${(parse(query) ?? 0) / 100}pt`,
      }}
      {items}
      onselect={(id) => apply(Number(id), close)}
      placeholder="폰트 크기"
      {recentIds}
    />
  {/snippet}
</ToolbarPanelDropdown>
