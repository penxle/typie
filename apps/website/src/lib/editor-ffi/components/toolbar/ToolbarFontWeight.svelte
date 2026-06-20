<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { SearchableDropdown } from '@typie/ui/components';
  import { SvelteMap } from 'svelte/reactivity';
  import { FontSpecimen } from '$lib/components';
  import { weightSpecimenFallbacks } from '$lib/components/font-specimen';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { values } from '$lib/editor-ffi/values';

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
        const fonts = [
          ...new Map(
            family.fonts
              .filter((f) => f.state === 'ACTIVE' || (currentWeight !== undefined && f.weight === currentWeight))
              .toSorted((a, b) => a.weight - b.weight)
              .map((f) => [f.weight, f]),
          ).values(),
        ];
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

  const weightItems = $derived.by(() => {
    const items = currentFontFamilyAndFonts.fonts.map((font) => ({
      value: font.weight,
      label:
        values.fontWeight.find((f) => f.value === font.weight)?.label ??
        (font.subfamilyDisplayName ? `${font.subfamilyDisplayName} (${font.weight})` : String(font.weight)),
    }));

    if (currentWeight != null && items.every((w) => w.value !== currentWeight)) {
      items.push({ value: currentWeight, label: values.fontWeight.find((f) => f.value === currentWeight)?.label ?? String(currentWeight) });
      items.sort((a, b) => a.value - b.value);
    }

    return items;
  });

  const weightFontIdMap = $derived(
    new Map(
      currentFontFamilyAndFonts.fonts.filter((f): f is typeof f & { id: string } => 'id' in f && !!f.id).map((f) => [f.weight, f.id]),
    ),
  );
</script>

<SearchableDropdown
  style={css.raw({ width: '100px' })}
  getLabel={(value) => {
    const font = currentFontFamilyAndFonts.fonts.find((f) => f.weight === value);
    return (
      values.fontWeight.find((f) => f.value === value)?.label ??
      (font?.subfamilyDisplayName ? `${font.subfamilyDisplayName} (${value})` : '(알 수 없는 굵기)')
    );
  }}
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
