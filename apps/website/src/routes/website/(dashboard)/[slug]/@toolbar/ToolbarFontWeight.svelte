<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { defaultValues, values } from '@typie/ui/tiptap';
  import { fragment, graphql } from '$graphql';
  import ToolbarDropdownButton from './ToolbarDropdownButton.svelte';
  import ToolbarDropdownMenu from './ToolbarDropdownMenu.svelte';
  import ToolbarDropdownMenuItem from './ToolbarDropdownMenuItem.svelte';
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
</script>

<ToolbarDropdownButton
  style={css.raw({ width: '100px' })}
  chevron
  disabled={!editor?.current.can().chain().setFontFamily(defaultValues.fontFamily).run()}
  label="폰트 두께"
  onEscape={() => editor?.current.commands.focus()}
  size="small"
>
  {#snippet anchor()}
    <div class={css({ flexGrow: '1', fontSize: '14px', color: 'text.subtle', lineClamp: '1' })}>
      {values.fontWeight.find(({ value }) => value === (editor?.current.getAttributes('text_style').fontWeight ?? defaultValues.fontWeight))
        ?.label}
    </div>
  {/snippet}

  {#snippet floating({ close, opened })}
    <ToolbarDropdownMenu onclose={close} {opened}>
      {#each currentFontFamilyAndWeights.weights as weight (weight)}
        <ToolbarDropdownMenuItem
          active={(editor?.current.getAttributes('text_style').fontWeight ?? defaultValues.fontWeight) === weight}
          onclick={() => {
            editor?.current
              .chain()
              .focus()
              .setFontWeight(weight as never)
              .run();
            close();
          }}
        >
          <div style:font-family={currentFontFamilyAndWeights.family} style:font-weight={weight}>
            {values.fontWeight.find(({ value }) => value === weight)?.label || weight}
          </div>
        </ToolbarDropdownMenuItem>
      {/each}
    </ToolbarDropdownMenu>
  {/snippet}
</ToolbarDropdownButton>
