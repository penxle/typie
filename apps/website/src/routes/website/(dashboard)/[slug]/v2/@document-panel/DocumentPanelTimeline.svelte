<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import ClockRewindIcon from '~icons/lucide/clock-arrow-up';
  import { graphql } from '$mearie';
  import type { Editor } from '$lib/editor-ffi/editor.svelte';
  import type { DocumentPanelV2Timeline_document$key } from '$mearie';

  type Props = {
    document$key: DocumentPanelV2Timeline_document$key;
    editor: Editor | undefined;
  };

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  let { document$key, editor: _editor }: Props = $props();

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const document = createFragment(
    graphql(`
      fragment DocumentPanelV2Timeline_document on Document {
        id

        entity {
          id
          slug
        }
      }
    `),
    () => document$key,
  );
</script>

<div
  class={flex({
    flexDirection: 'column',
    minWidth: 'var(--min-width)',
    width: 'var(--width)',
    maxWidth: 'var(--max-width)',
    height: 'full',
  })}
>
  <div
    class={flex({
      flexShrink: '0',
      alignItems: 'center',
      height: '41px',
      paddingX: '20px',
      fontSize: '13px',
      fontWeight: 'semibold',
      color: 'text.subtle',
      borderBottomWidth: '1px',
      borderColor: 'surface.muted',
    })}
  >
    타임라인
  </div>

  <div
    class={flex({
      flexDirection: 'column',
      alignItems: 'center',
      justifyContent: 'center',
      gap: '20px',
      paddingY: '60px',
    })}
  >
    <div
      class={center({
        size: '64px',
        borderRadius: '16px',
        backgroundColor: 'surface.muted',
        color: 'text.faint',
      })}
    >
      <Icon icon={ClockRewindIcon} size={28} />
    </div>

    <p class={css({ fontSize: '13px', color: 'text.faint', textAlign: 'center' })}>v2에서 곧 지원 예정이에요</p>
  </div>
</div>
