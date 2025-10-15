<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { SearchableDropdown } from '@typie/ui/components';
  import { defaultValues, values } from '@typie/ui/tiptap';
  import { fragment, graphql } from '$graphql';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '@typie/ui/utils';
  import type { Editor_BottomToolbar_FontWeight_user } from '$graphql';

  type Props = {
    $user: Editor_BottomToolbar_FontWeight_user;
    editor?: Ref<Editor>;
  };

  let { $user: _user, editor }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment Editor_BottomToolbar_FontWeight_user on User {
        id

        fontFamilies {
          id
          name

          fonts {
            id
            weight
          }
        }
      }
    `),
  );

  const currentFontFamilyAndWeights = $derived.by(() => {
    const defaultFontFamilyAndWeights = {
      family: defaultValues.fontFamily,
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      weights: values.fontFamily.find((f) => f.value === defaultValues.fontFamily)!.weights.toSorted((a, b) => a - b),
    };

    const fontOrFontIdOrFontFamilyId = editor?.current.getAttributes('text_style').fontFamily;
    if (!fontOrFontIdOrFontFamilyId) return defaultFontFamilyAndWeights;

    const systemFontFamily = values.fontFamily.find((f) => f.value === fontOrFontIdOrFontFamilyId);
    if (systemFontFamily) {
      return {
        family: systemFontFamily.value,
        weights: systemFontFamily.weights.toSorted((a, b) => a - b),
      };
    }

    const userFonts = $user.fontFamilies.flatMap((f) => f.fonts);
    if (userFonts.length === 0) return defaultFontFamilyAndWeights;

    const userFontFamily = $user.fontFamilies.find(
      ({ id, fonts }) => id === fontOrFontIdOrFontFamilyId || fonts.some(({ id }) => id === fontOrFontIdOrFontFamilyId),
    );
    if (!userFontFamily) return defaultFontFamilyAndWeights;

    return {
      family: userFontFamily.id,
      weights: userFontFamily.fonts.map((f) => f.weight).toSorted((a, b) => a - b),
    };
  });

  const currentWeight = $derived(editor?.current.getAttributes('text_style').fontWeight ?? defaultValues.fontWeight);

  const weightItems = $derived(
    currentFontFamilyAndWeights.weights.map((weight) => ({
      value: weight,
      label: values.fontWeight.find(({ value }) => value === weight)?.label || String(weight),
    })),
  );
</script>

<SearchableDropdown
  style={css.raw({ width: '100px' })}
  disabled={!editor?.current.can().chain().setFontFamily(defaultValues.fontFamily).run()}
  getLabel={(value) => {
    const item = weightItems.find((w) => w.value === value);
    return item?.label ?? '(알 수 없는 굵기)';
  }}
  items={weightItems}
  label="폰트 굵기"
  onEscape={() => editor?.current.commands.focus()}
  onchange={(weight, options) => {
    const chain = editor?.current.chain().setFontWeight(weight);
    if (options?.shouldFocus) {
      chain?.focus();
    }
    chain?.run();
  }}
  value={currentWeight}
>
  {#snippet renderItem(item)}
    <div style:font-family={currentFontFamilyAndWeights.family} style:font-weight={item.value}>
      {item.label}
    </div>
  {/snippet}
</SearchableDropdown>
