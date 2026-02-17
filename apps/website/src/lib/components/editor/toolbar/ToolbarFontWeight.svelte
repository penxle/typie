<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { SearchableDropdown } from '@typie/ui/components';
  import { SvelteMap } from 'svelte/reactivity';
  import { FontSpecimen } from '$lib/components';
  import { getEditorContext } from '$lib/editor/context.svelte';
  import { values } from '$lib/editor/values';

  const { editor } = getEditorContext();

  const fontWeightAttr = $derived(editor.getAttr('font_weight'));
  const fontWeightValues = $derived(fontWeightAttr?.values.filter((v): v is number => v != null) ?? []);
  const currentWeight = $derived(fontWeightValues.length === 1 ? fontWeightValues[0] : undefined);

  const currentFontFamilyAndFonts = $derived.by(() => {
    const fontFamilyAttr = editor.getAttr('font_family');
    const fontFamilyValues = fontFamilyAttr?.values.filter((v): v is string => v != null) ?? [];

    if (fontFamilyValues.length === 1) {
      const family = editor.fontFamilies.find((f) => f.familyName === fontFamilyValues[0]);
      if (family) {
        const fonts = [
          ...new Map(
            family.fonts
              .filter((f) => f.state === 'ACTIVE' || fontWeightValues.includes(f.weight))
              .toSorted((a, b) => a.weight - b.weight)
              .map((f) => [f.weight, f]),
          ).values(),
        ];
        return { family: family.familyName, fonts };
      }
    }

    const fontsByWeight = new SvelteMap<number, { weight: number; subfamilyDisplayName?: string | null }>();
    for (const familyName of fontFamilyValues) {
      const family = editor.fontFamilies.find((f) => f.familyName === familyName);
      if (family) {
        for (const font of family.fonts) {
          if (font.state === 'ACTIVE') {
            fontsByWeight.set(font.weight, font);
          }
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
        values.fontWeight[font.weight] ??
        (font.subfamilyDisplayName ? `${font.subfamilyDisplayName} (${font.weight})` : String(font.weight)),
    }));

    if (currentWeight != null && !items.some((w) => w.value === currentWeight)) {
      items.push({ value: currentWeight, label: values.fontWeight[currentWeight] ?? String(currentWeight) });
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
  disabled={!editor.can('toggleStyle')}
  getLabel={(value) => {
    const font = currentFontFamilyAndFonts.fonts.find((f) => f.weight === value);
    return values.fontWeight[value] ?? (font?.subfamilyDisplayName ? `${font.subfamilyDisplayName} (${value})` : '(알 수 없는 굵기)');
  }}
  items={weightItems}
  label="폰트 굵기"
  onEscape={() => editor.focus()}
  onchange={(weight, options) => {
    editor.dispatch({ type: 'toggleStyle', style: { type: 'font_weight', weight } });
    if (options?.shouldFocus) {
      editor.focus();
    }
  }}
  placeholder="-"
  value={currentWeight}
>
  {#snippet renderItem(item)}
    <FontSpecimen fontId={weightFontIdMap.get(item.value)} text={item.label} weight={item.value} />
  {/snippet}
</SearchableDropdown>
