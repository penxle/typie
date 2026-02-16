<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { SearchableDropdown } from '@typie/ui/components';
  import { getEditorContext } from '$lib/editor/context.svelte';

  const { editor } = getEditorContext();

  const fontFamilies = [
    { label: '프리텐다드', value: 'Pretendard', weights: [100, 200, 300, 400, 500, 600, 700, 800, 900] },
    { label: '코펍월드돋움', value: 'KoPubWorldDotum', weights: [500, 700] },
    { label: '나눔바른고딕', value: 'NanumBarunGothic', weights: [400, 700] },
    { label: '리디바탕', value: 'RIDIBatang', weights: [400] },
    { label: '코펍월드바탕', value: 'KoPubWorldBatang', weights: [500, 700] },
    { label: '나눔명조', value: 'NanumMyeongjo', weights: [400, 700] },
  ];

  const defaultFontWeight = 400;

  const fontFamilyAttr = $derived(editor.getAttr('font_family'));
  const fontFamilyValues = $derived(fontFamilyAttr?.values.filter((v): v is string => v != null) ?? []);
  const currentFontFamilyValue = $derived(fontFamilyValues.length === 1 ? fontFamilyValues[0] : undefined);

  const allFontFamilies = $derived(fontFamilies.map((f) => ({ value: f.value, label: f.label })));

  const getDefaultWeight = (fontFamilyValue: string, fontWeight: number) => {
    const systemFontFamily = fontFamilies.find((f) => f.value === fontFamilyValue);
    if (!systemFontFamily) return defaultFontWeight;

    const weights = systemFontFamily.weights.toSorted((a, b) => a - b);
    if (weights.length === 0) return defaultFontWeight;

    if (weights.includes(fontWeight)) {
      return fontWeight;
    }

    let closest = weights[0];
    let minDiff = Math.abs(fontWeight - weights[0]);

    for (const weight of weights) {
      const diff = Math.abs(fontWeight - weight);
      if (diff < minDiff) {
        minDiff = diff;
        closest = weight;
      }
    }

    return closest;
  };
</script>

<SearchableDropdown
  style={css.raw({ width: '120px' })}
  disabled={!editor.can('toggleStyle')}
  getLabel={(value) => {
    const item = allFontFamilies.find((f) => f.value === value);
    return item?.label ?? '(알 수 없는 폰트)';
  }}
  items={allFontFamilies}
  label="폰트 패밀리"
  onEscape={() => editor.focus()}
  onchange={(fontFamilyValue, options) => {
    const weightAttr = editor.getAttr('font_weight');
    const weightValues = weightAttr?.values.filter((v): v is number => v != null) ?? [];
    const currentWeight = weightValues.length === 1 ? weightValues[0] : defaultFontWeight;
    const defaultWeight = getDefaultWeight(fontFamilyValue, currentWeight);

    editor.dispatch({ type: 'toggleStyle', style: { type: 'font_family', family: fontFamilyValue } });
    editor.dispatch({ type: 'toggleStyle', style: { type: 'font_weight', weight: defaultWeight } });
    if (options?.shouldFocus) {
      editor.focus();
    }
  }}
  placeholder="-"
  value={currentFontFamilyValue}
>
  {#snippet renderItem(item)}
    <div style:font-family={item.value}>{item.label}</div>
  {/snippet}
</SearchableDropdown>
