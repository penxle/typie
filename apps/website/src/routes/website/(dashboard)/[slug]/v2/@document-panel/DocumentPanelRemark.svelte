<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import MessageSquareTextIcon from '~icons/lucide/message-square-text';
  import type { Editor } from '$lib/editor-ffi/editor.svelte';

  type Props = {
    editor: Editor | undefined;
  };

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  let { editor: _editor }: Props = $props();
</script>

<div class={flex({ flexDirection: 'column', flexGrow: '1', height: 'full', overflow: 'hidden' })}>
  <div
    class={flex({
      justifyContent: 'space-between',
      alignItems: 'center',
      height: '41px',
      paddingX: '20px',
      flexShrink: '0',
      borderBottomWidth: '1px',
      borderColor: 'surface.muted',
    })}
  >
    <div class={flex({ alignItems: 'center', gap: '6px', fontWeight: 'semibold' })}>
      <div class={css({ fontSize: '13px', color: 'text.subtle' })}>코멘트</div>
    </div>
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
      <Icon icon={MessageSquareTextIcon} size={28} />
    </div>

    <p class={css({ fontSize: '13px', color: 'text.faint', textAlign: 'center' })}>v2에서 곧 지원 예정이에요</p>
  </div>
</div>
