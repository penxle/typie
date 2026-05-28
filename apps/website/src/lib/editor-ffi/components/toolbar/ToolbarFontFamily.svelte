<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, SearchableDropdown } from '@typie/ui/components';
  import { SvelteMap } from 'svelte/reactivity';
  import PlusIcon from '~icons/lucide/plus';
  import { FontSpecimen } from '$lib/components';
  import { familySpecimenFallbacks } from '$lib/components/font-specimen';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';

  type Font = { id?: string | null; weight: number; subfamilyDisplayName?: string | null; state: string };
  type FontFamily = { id: string; familyName: string; displayName: string; state: string; fonts: readonly Font[] };

  type Props = {
    fontFamilies?: readonly FontFamily[];
    onUploadClick?: () => void;
  };

  let { fontFamilies = [], onUploadClick }: Props = $props();

  const ctx = getEditorContext();

  const currentFontFamilyValue = $derived(
    ctx.editor?.modifierState?.font_family?.type === 'uniform' ? ctx.editor.modifierState.font_family.value.value : undefined,
  );

  const activeFamilies = $derived(fontFamilies.filter((f) => f.state === 'ACTIVE'));

  const fontFamilyItems = $derived.by(() => {
    if (currentFontFamilyValue && !activeFamilies.some((f) => f.familyName === currentFontFamilyValue)) {
      const current = fontFamilies.find((f) => f.familyName === currentFontFamilyValue);
      if (current) {
        return [...activeFamilies, current];
      }
    }
    return activeFamilies;
  });

  const representativeFontMap = $derived.by(() => {
    const map = new SvelteMap<string, Font | null>();
    for (const family of fontFamilies) {
      const active = family.fonts.filter((f) => f.state === 'ACTIVE');
      if (active.length === 0) {
        map.set(family.familyName, null);
      } else {
        map.set(
          family.familyName,
          active.reduce((prev, curr) => {
            const prevDiff = Math.abs(prev.weight - 400);
            const currDiff = Math.abs(curr.weight - 400);
            if (currDiff < prevDiff) return curr;
            if (currDiff === prevDiff && curr.weight > prev.weight) return curr;
            return prev;
          }),
        );
      }
    }
    return map;
  });
</script>

{#snippet uploadFontFamilyItem()}
  <div class={flex({ alignItems: 'center', gap: '4px' })}>
    <Icon
      style={css.raw({ color: 'text.faint', transitionProperty: '[none]', _groupHover: { color: 'text.brand' } })}
      icon={PlusIcon}
      size={14}
    />
    <span class={css({ color: 'text.subtle', _groupHover: { color: 'text.brand' } })}>직접 업로드</span>
  </div>
{/snippet}

<SearchableDropdown
  style={css.raw({ width: '120px' })}
  extraItems={onUploadClick
    ? [
        {
          onclick: () => onUploadClick?.(),
          content: uploadFontFamilyItem,
        },
      ]
    : undefined}
  getLabel={(value) => fontFamilies.find((f) => f.familyName === value)?.displayName ?? '(알 수 없는 폰트)'}
  items={fontFamilyItems.map((f) => ({ value: f.familyName, label: f.displayName }))}
  label="폰트 패밀리"
  onEscape={() => ctx.editor?.focus()}
  onchange={(familyName, options) => {
    ctx.editor?.enqueue({ type: 'modifier', op: { type: 'set', modifier: { type: 'font_family', value: familyName } } });
    if (options?.shouldFocus) {
      ctx.editor?.focus();
    }
  }}
  placeholder="-"
  value={currentFontFamilyValue}
>
  {#snippet renderItem(item)}
    {@const font = representativeFontMap.get(item.value)}
    <FontSpecimen
      fallbacks={familySpecimenFallbacks(item.label, item.value)}
      fontId={font?.id ?? undefined}
      text={item.label}
      weight={font?.weight}
    />
  {/snippet}
</SearchableDropdown>
