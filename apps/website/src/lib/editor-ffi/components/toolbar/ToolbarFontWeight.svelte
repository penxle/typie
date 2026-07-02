<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { SearchableDropdown } from '@typie/ui/components';
  import { SvelteMap } from 'svelte/reactivity';
  import { FontSpecimen } from '$lib/components';
  import { weightSpecimenFallbacks } from '$lib/components/font-specimen';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { values } from '$lib/editor-ffi/values';
  import { activeFontsByWeight, fontWeightItemsForFonts, fontWeightValueLabel } from '$lib/font-weight';

  type Font = { id?: string | null; weight: number; subfamilyDisplayName?: string | null; state: string };
  type FontFamily = { familyName: string; fonts: readonly Font[] };

  type Props = {
    fontFamilies?: readonly FontFamily[];
  };

  let { fontFamilies = [] }: Props = $props();

  const ctx = getEditorContext();

  const currentWeight = $derived(
    ctx.editor?.modifierState?.font_weight?.type === 'uniform' ? ctx.editor.modifierState.font_weight.value.value : undefined,
  );

  const currentFontFamilyValue = $derived(
    ctx.editor?.modifierState?.font_family?.type === 'uniform' ? ctx.editor.modifierState.font_family.value.value : undefined,
  );

  const currentFontFamilyAndFonts = $derived.by(() => {
    if (currentFontFamilyValue) {
      const family = fontFamilies.find((f) => f.familyName === currentFontFamilyValue);
      if (family) {
        const fonts = activeFontsByWeight(family.fonts);
        return { family: family.familyName, fonts };
      }
    }

    const fontsByWeight = new SvelteMap<number, Font>();
    for (const family of fontFamilies) {
      for (const font of family.fonts) {
        if (font.state === 'ACTIVE') {
          fontsByWeight.set(font.weight, font);
        }
      }
    }

    return {
      family: undefined as string | undefined,
      fonts: [...fontsByWeight.values()].toSorted((a, b) => a.weight - b.weight),
    };
  });

  const weightItems = $derived(fontWeightItemsForFonts(currentFontFamilyAndFonts.fonts, values.fontWeight));

  const weightFontIdMap = $derived(
    new Map(
      currentFontFamilyAndFonts.fonts.filter((f): f is typeof f & { id: string } => 'id' in f && !!f.id).map((f) => [f.weight, f.id]),
    ),
  );
</script>

<SearchableDropdown
  style={css.raw({ width: '100px' })}
  getLabel={(value) => fontWeightValueLabel(currentFontFamilyAndFonts.fonts, values.fontWeight, value)}
  items={weightItems}
  label="폰트 굵기"
  onEscape={() => ctx.editor?.focus()}
  onchange={(weight, options) => {
    ctx.editor?.enqueue({ type: 'modifier', op: { type: 'set', modifier: { type: 'font_weight', value: weight } } });
    if (options?.shouldFocus) {
      ctx.editor?.focus();
    }
  }}
  placeholder="-"
  value={currentWeight}
>
  {#snippet renderItem(item)}
    {@const font = currentFontFamilyAndFonts.fonts.find((candidate) => candidate.weight === item.value)}
    <FontSpecimen
      fallbacks={weightSpecimenFallbacks(item.label, font?.subfamilyDisplayName, item.value)}
      fontId={weightFontIdMap.get(item.value)}
      text={item.label}
      weight={item.value}
    />
  {/snippet}
</SearchableDropdown>
