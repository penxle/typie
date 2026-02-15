<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { SearchableDropdown } from '@typie/ui/components';
  import { getEditorContext } from '$lib/editor/context.svelte';

  const { editor } = getEditorContext();

  const fontWeights = [
    { label: '가장 가늘게', value: 100 },
    { label: '아주 가늘게', value: 200 },
    { label: '가늘게', value: 300 },
    { label: '보통', value: 400 },
    { label: '중간', value: 500 },
    { label: '약간 굵게', value: 600 },
    { label: '굵게', value: 700 },
    { label: '아주 굵게', value: 800 },
    { label: '가장 굵게', value: 900 },
  ];

  const fontFamilies = [
    { label: '프리텐다드', value: 'Pretendard', weights: [100, 200, 300, 400, 500, 600, 700, 800, 900] },
    { label: '코펍월드돋움', value: 'KoPubWorldDotum', weights: [500, 700] },
    { label: '나눔바른고딕', value: 'NanumBarunGothic', weights: [400, 700] },
    { label: '리디바탕', value: 'RIDIBatang', weights: [400] },
    { label: '코펍월드바탕', value: 'KoPubWorldBatang', weights: [500, 700] },
    { label: '나눔명조', value: 'NanumMyeongjo', weights: [400, 700] },
  ];

  const defaultFontFamily = 'Pretendard';

  const currentFontFamilyAndWeights = $derived.by(() => {
    const defaultFontFamilyWeights = fontFamilies.find((f) => f.value === defaultFontFamily)?.weights ?? [400];
    const defaultFontFamilyAndWeights = {
      family: defaultFontFamily as string | undefined,
      weights: defaultFontFamilyWeights.toSorted((a, b) => a - b),
    };

    const fontFamilyAttr = editor.getAttr('font_family');
    const fontFamilyValues = fontFamilyAttr?.values.filter((v): v is string => v != null) ?? [];

    if (fontFamilyValues.length === 0) return defaultFontFamilyAndWeights;

    if (fontFamilyValues.length === 1) {
      const fontFamily = fontFamilyValues[0];
      const systemFontFamily = fontFamilies.find((f) => f.value === fontFamily);
      if (systemFontFamily) {
        return {
          family: systemFontFamily.value as string | undefined,
          weights: systemFontFamily.weights.toSorted((a, b) => a - b),
        };
      }
      return defaultFontFamilyAndWeights;
    }

    const allWeights: number[] = [];
    for (const familyValue of fontFamilyValues) {
      const systemFamily = fontFamilies.find((f) => f.value === familyValue);
      if (systemFamily) {
        for (const w of systemFamily.weights) {
          if (!allWeights.includes(w)) {
            allWeights.push(w);
          }
        }
      }
    }

    if (allWeights.length === 0) return defaultFontFamilyAndWeights;

    return {
      family: undefined as string | undefined,
      weights: [...allWeights].toSorted((a, b) => a - b),
    };
  });

  const fontWeightAttr = $derived(editor.getAttr('font_weight'));
  const fontWeightValues = $derived(fontWeightAttr?.values.filter((v): v is number => v != null) ?? []);
  const currentWeight = $derived(fontWeightValues.length === 1 ? fontWeightValues[0] : undefined);

  const weightItems = $derived(
    currentFontFamilyAndWeights.weights.map((weight) => ({
      value: weight,
      label: fontWeights.find(({ value }) => value === weight)?.label || String(weight),
    })),
  );
</script>

<SearchableDropdown
  style={css.raw({ width: '100px' })}
  disabled={!editor.can('toggleStyle')}
  getLabel={(value) => {
    const item = weightItems.find((w) => w.value === value);
    return item?.label ?? '(알 수 없는 굵기)';
  }}
  items={weightItems}
  label="폰트 굵기"
  onEscape={() => editor.focus()}
  onchange={(weight) => {
    editor.focus().dispatch({ type: 'toggleStyle', style: { type: 'font_weight', weight } });
  }}
  placeholder="-"
  value={currentWeight}
>
  {#snippet renderItem(item)}
    <div style:font-family={currentFontFamilyAndWeights.family} style:font-weight={item.value}>
      {item.label}
    </div>
  {/snippet}
</SearchableDropdown>
