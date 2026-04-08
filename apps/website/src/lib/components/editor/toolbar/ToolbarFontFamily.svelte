<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, SearchableDropdown } from '@typie/ui/components';
  import PlusIcon from '~icons/lucide/plus';
  import { FontSpecimen } from '$lib/components';
  import { familySpecimenFallbacks } from '$lib/components/font-specimen';
  import { getEditorContext } from '$lib/editor/context.svelte';
  import { getRepresentativeFont } from '$lib/editor/fonts';

  type Props = {
    onUploadClick?: () => void;
  };

  let { onUploadClick }: Props = $props();

  const { editor } = getEditorContext();

  const fontFamilyAttr = $derived(editor.getAttr('font_family'));
  const fontFamilyValues = $derived(fontFamilyAttr?.values.filter((v): v is string => v != null) ?? []);
  const currentFontFamilyValue = $derived(fontFamilyValues.length === 1 ? fontFamilyValues[0] : undefined);

  const fontFamilyItems = $derived.by(() => {
    const active = editor.fontFamilies.filter((f) => f.state === 'ACTIVE');
    if (currentFontFamilyValue && !active.some((f) => f.familyName === currentFontFamilyValue)) {
      const current = editor.fontFamilies.find((f) => f.familyName === currentFontFamilyValue);
      if (current) {
        return [...active, current];
      }
    }
    return active;
  });

  const representativeFontMap = $derived(new Map(editor.fontFamilies.map((f) => [f.familyName, getRepresentativeFont(f.fonts)])));
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
  disabled={!editor.can('toggleStyle')}
  extraItems={onUploadClick
    ? [
        {
          onclick: () => onUploadClick?.(),
          content: uploadFontFamilyItem,
        },
      ]
    : undefined}
  getLabel={(value) => editor.fontFamilies.find((f) => f.familyName === value)?.displayName ?? '(알 수 없는 폰트)'}
  items={fontFamilyItems.map((f) => ({ value: f.familyName, label: f.displayName }))}
  label="폰트 패밀리"
  onEscape={() => editor.focus()}
  onchange={(familyName, options) => {
    editor.dispatch({ type: 'toggleStyle', style: { type: 'font_family', family: familyName } });
    if (options?.shouldFocus) {
      editor.focus();
    }
  }}
  placeholder="-"
  value={currentFontFamilyValue}
>
  {#snippet renderItem(item)}
    {@const font = representativeFontMap.get(item.value)}
    <FontSpecimen fallbacks={familySpecimenFallbacks(item.label, item.value)} fontId={font?.id} text={item.label} weight={font?.weight} />
  {/snippet}
</SearchableDropdown>
