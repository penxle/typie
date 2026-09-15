<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getAppContext } from '@typie/ui/context';
  import { SvelteMap } from 'svelte/reactivity';
  import { FontSpecimen } from '$lib/components';
  import { weightSpecimenFallbacks } from '$lib/components/font-specimen';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { values } from '$lib/editor-ffi/values';
  import { activeFontsByWeight, fontWeightItemsForFonts, fontWeightValueLabel } from '$lib/font-weight';
  import { toolbarRecent } from './toolbar-recent.svelte';
  import ToolbarPanel from './ToolbarPanel.svelte';
  import ToolbarPanelDropdown from './ToolbarPanelDropdown.svelte';
  import type { ToolbarPanelItem } from './ToolbarPanel.svelte';

  type Font = { id?: string | null; weight: number; subfamilyDisplayName?: string | null; state: string };
  type FontFamily = { id: string; familyName: string; displayName: string; state: string; fonts: readonly Font[] };

  type Props = {
    fontFamilies?: readonly FontFamily[];
    disabled?: boolean;
  };

  let { fontFamilies = [], disabled = false }: Props = $props();

  const app = getAppContext();
  const ctx = getEditorContext();
  const recent = toolbarRecent(app.userId);

  const currentWeight = $derived(
    ctx.editor?.modifierState?.font_weight?.type === 'uniform' ? ctx.editor.modifierState.font_weight.value.value : undefined,
  );

  const currentFontFamilyValue = $derived(
    ctx.editor?.modifierState?.font_family?.type === 'uniform' ? ctx.editor.modifierState.font_family.value.value : undefined,
  );

  const currentFonts = $derived.by(() => {
    if (currentFontFamilyValue) {
      const family = fontFamilies.find((f) => f.familyName === currentFontFamilyValue);
      if (family) return activeFontsByWeight(family.fonts);
    }

    const fontsByWeight = new SvelteMap<number, Font>();
    for (const family of fontFamilies) {
      for (const font of family.fonts) {
        if (font.state === 'ACTIVE') {
          fontsByWeight.set(font.weight, font);
        }
      }
    }
    return [...fontsByWeight.values()].toSorted((a, b) => a.weight - b.weight);
  });

  const items = $derived<ToolbarPanelItem[]>(
    fontWeightItemsForFonts(currentFonts, values.fontWeight).map((item) => ({
      id: String(item.value),
      label: item.label,
      keywords: [String(item.value)],
      selected: item.value === currentWeight,
    })),
  );

  const apply = (id: string, close: () => void) => {
    ctx.editor?.enqueue({ type: 'modifier', op: { type: 'set', modifier: { type: 'font_weight', value: Number(id) } } });
    recent.remember('fontWeight', id);
    close();
    ctx.editor?.focus();
  };
</script>

<ToolbarPanelDropdown style={css.raw({ maxWidth: '180px' })} chevron {disabled} label="폰트 굵기">
  {#snippet anchor()}
    <span class={css({ paddingLeft: '2px', fontSize: '13px', fontWeight: 'medium', whiteSpace: 'nowrap', truncate: true })}>
      {currentWeight === undefined ? '-' : fontWeightValueLabel(currentFonts, values.fontWeight, currentWeight)}
    </span>
  {/snippet}

  {#snippet panel({ close })}
    <ToolbarPanel {items} onselect={(id) => apply(id, close)} placeholder="폰트 굵기" recentIds={recent.ids('fontWeight')} rowHeight={32}>
      {#snippet render(item)}
        {@const font = currentFonts.find((candidate) => String(candidate.weight) === item.id)}
        <span class={css({ minWidth: '0', overflow: 'hidden', whiteSpace: 'nowrap' })}>
          <FontSpecimen
            fallbacks={weightSpecimenFallbacks(item.label, font?.subfamilyDisplayName, Number(item.id))}
            fontId={font?.id ?? undefined}
            text={item.label}
            weight={Number(item.id)}
          />
        </span>
      {/snippet}
    </ToolbarPanel>
  {/snippet}
</ToolbarPanelDropdown>
