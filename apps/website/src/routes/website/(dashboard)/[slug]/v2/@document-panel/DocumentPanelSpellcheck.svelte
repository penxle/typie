<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import SpellCheckIcon from '~icons/lucide/spell-check';
  import { graphql } from '$mearie';
  import type { Editor } from '$lib/editor-ffi/editor.svelte';
  import type { DocumentPanelV2_Spellcheck_document$key, DocumentPanelV2_Spellcheck_user$key } from '$mearie';

  type Props = {
    document$key: DocumentPanelV2_Spellcheck_document$key;
    user$key: DocumentPanelV2_Spellcheck_user$key;
    editor: Editor | undefined;
  };

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  let { document$key, user$key, editor: _editor }: Props = $props();

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const document = createFragment(
    graphql(`
      fragment DocumentPanelV2_Spellcheck_document on Document {
        id
      }
    `),
    () => document$key,
  );

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const user = createFragment(
    graphql(`
      fragment DocumentPanelV2_Spellcheck_user on User {
        id
        subscription {
          id
        }
      }
    `),
    () => user$key,
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
    맞춤법 검사
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
      <Icon icon={SpellCheckIcon} size={28} />
    </div>

    <p class={css({ fontSize: '13px', color: 'text.faint', textAlign: 'center' })}>v2에서 곧 지원 예정이에요</p>
  </div>
</div>
