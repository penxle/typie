<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { defaultValues } from '@typie/ui/tiptap';
  import { fragment, graphql } from '$graphql';
  import ToolbarDropdownButton from './ToolbarDropdownButton.svelte';
  import ToolbarDropdownMenu from './ToolbarDropdownMenu.svelte';
  import ToolbarDropdownMenuItem from './ToolbarDropdownMenuItem.svelte';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '@typie/ui/utils';
  import type { Editor_BottomToolbar_FontWeight_site, Optional } from '$graphql';

  type Props = {
    $site: Optional<Editor_BottomToolbar_FontWeight_site>;
    editor?: Ref<Editor>;
  };

  let { $site: _site, editor }: Props = $props();

  const site = fragment(
    _site,
    graphql(`
      fragment Editor_BottomToolbar_FontWeight_site on Site {
        id

        fonts {
          id
          name
          weight
        }
      }
    `),
  );

  const weightLabels: Record<number, string> = {
    300: '가늘게',
    400: '보통',
    500: '약간 굵게',
  };

  const currentFontFamily = $derived(editor?.current.getAttributes('text_style').fontFamily ?? defaultValues.fontFamily);

  const currentFont = $derived.by(() => {
    if (!$site?.fonts || !currentFontFamily) return null;
    return $site.fonts.find((f) => f.id === currentFontFamily);
  });

  const availableWeights = $derived.by(() => {
    const font = currentFont;
    if (!font || !$site?.fonts) return [];

    return $site.fonts.filter((f) => f.name === font.name).sort((a, b) => a.weight - b.weight);
  });

  const currentWeightLabel = $derived.by(() => {
    const weight = currentFont?.weight || 400;
    return weightLabels[weight] || weight.toString();
  });

  const shouldShow = $derived.by(() => {
    return availableWeights.length > 1;
  });
</script>

{#if shouldShow}
  <ToolbarDropdownButton
    style={css.raw({ width: '100px' })}
    chevron
    disabled={!editor?.current.can().chain().focus().setFontFamily(defaultValues.fontFamily).run()}
    label="폰트 두께"
    size="small"
  >
    {#snippet anchor()}
      <div class={css({ flexGrow: '1', fontSize: '14px', color: 'text.subtle', lineClamp: '1' })}>
        {currentWeightLabel}
      </div>
    {/snippet}

    {#snippet floating({ close })}
      <ToolbarDropdownMenu>
        {#each availableWeights as font (font.id)}
          <ToolbarDropdownMenuItem
            active={currentFont?.id === font.id}
            onclick={() => {
              editor?.current
                .chain()
                .focus()
                .setFontFamily(font.id as never)
                .run();
              close();
            }}
          >
            <div style:font-family={font.id}>{weightLabels[font.weight] || font.weight}</div>
          </ToolbarDropdownMenuItem>
        {/each}
      </ToolbarDropdownMenu>
    {/snippet}
  </ToolbarDropdownButton>
{/if}
