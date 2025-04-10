<script lang="ts">
  import { getText } from '@tiptap/core';
  import { scale } from 'svelte/transition';
  import { textSerializers } from '@/pm/serializer';
  import IconType from '~icons/lucide/type';
  import { createFloatingActions } from '$lib/actions';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import type { Editor } from '@tiptap/core';
  import type { Transaction } from '@tiptap/pm/state';
  import type { Ref } from '$lib/utils';

  type Props = {
    editor?: Ref<Editor>;
  };

  let { editor }: Props = $props();

  let open = $state(false);
  let text = $state('');

  const countWithWhitespace = $derived(text.replaceAll(/\s+/g, ' ').trim().length);
  const countWithoutWhitespace = $derived(text.replaceAll(/\s/g, '').length);
  const countWithoutWhitespaceAndPunctuation = $derived(text.replaceAll(/[\s\p{P}]/gu, '').length);

  const { anchor, floating } = createFloatingActions({
    placement: 'top',
    offset: 14,
  });

  const handler = ({ editor, transaction }: { editor: Editor; transaction: Transaction }) => {
    if (transaction.docChanged) {
      text = getText(editor.state.doc, {
        blockSeparator: '\n',
        textSerializers,
      });
    }
  };

  $effect(() => {
    editor?.current.on('transaction', handler);

    return () => {
      editor?.current.off('transaction', handler);
    };
  });
</script>

<div
  class={flex({ alignItems: 'center', gap: '6px' })}
  onmouseenter={() => (open = true)}
  onmouseleave={() => (open = false)}
  role="presentation"
  use:anchor
>
  <Icon style={{ color: 'gray.500' }} icon={IconType} size={14} />
  <div class={css({ fontSize: '14px' })}>총 {countWithWhitespace}자</div>
</div>

{#if open}
  <div
    class={css({ borderWidth: '1px', borderRadius: '4px', paddingX: '12px', paddingY: '8px', backgroundColor: 'white' })}
    use:floating
    transition:scale={{ start: 0.95, duration: 200 }}
  >
    <dl class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
      <dt class={css({ color: 'gray.500' })}>공백 포함</dt>
      <dd class={css({ fontWeight: 'medium' })}>{countWithWhitespace}자</dd>
    </dl>

    <dl class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
      <dt class={css({ color: 'gray.500' })}>공백 미포함</dt>
      <dd class={css({ fontWeight: 'medium' })}>{countWithoutWhitespace}자</dd>
    </dl>

    <dl class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
      <dt class={css({ color: 'gray.500' })}>공백/부호 미포함</dt>
      <dd class={css({ fontWeight: 'medium' })}>{countWithoutWhitespaceAndPunctuation}자</dd>
    </dl>
  </div>
{/if}
