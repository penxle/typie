<script lang="ts">
  import { getText } from '@tiptap/core';
  import { fly } from 'svelte/transition';
  import { textSerializers } from '@/pm/serializer';
  import IconChevronRight from '~icons/lucide/chevron-right';
  import IconType from '~icons/lucide/type';
  import { Icon } from '$lib/components';
  import { comma } from '$lib/utils';
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

  const countWithWhitespace = $derived([...text.replaceAll(/\s+/g, ' ').trim()].length);
  const countWithoutWhitespace = $derived([...text.replaceAll(/\s/g, '').trim()].length);
  const countWithoutWhitespaceAndPunctuation = $derived([...text.replaceAll(/[\s\p{P}]/gu, '').trim()].length);

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

<details class={flex({ flexDirection: 'column', marginBottom: open ? '12px' : '8px' })} bind:open>
  <summary class={flex({ alignItems: 'center', gap: '4px', cursor: 'pointer', marginBottom: open ? '8px' : '0', userSelect: 'none' })}>
    <Icon style={{ color: 'text.faint' }} icon={IconType} size={12} />
    <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.subtle' })}>글자 수</div>
    <Icon style={css.raw({ color: 'text.faint', transform: open ? 'rotate(90deg)' : 'rotate(0deg)' })} icon={IconChevronRight} size={14} />
    <div class={css({ flexGrow: '1' })}></div>
    {#if !open}
      <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.subtle' })} in:fly={{ y: 2, duration: 150 }}>
        {comma(countWithWhitespace)}자
      </div>
    {/if}
  </summary>

  {#if open}
    <div class={flex({ flexDirection: 'column', gap: '2px' })} in:fly={{ y: -2, duration: 150 }}>
      <dl class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
        <dt class={css({ color: 'text.faint' })}>공백 포함</dt>
        <dd class={css({ fontWeight: 'medium', color: 'text.subtle' })}>{comma(countWithWhitespace)}자</dd>
      </dl>

      <dl class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
        <dt class={css({ color: 'text.faint' })}>공백 미포함</dt>
        <dd class={css({ fontWeight: 'medium', color: 'text.subtle' })}>{comma(countWithoutWhitespace)}자</dd>
      </dl>

      <dl class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
        <dt class={css({ color: 'text.faint' })}>공백/부호 미포함</dt>
        <dd class={css({ fontWeight: 'medium', color: 'text.subtle' })}>{comma(countWithoutWhitespaceAndPunctuation)}자</dd>
      </dl>
    </div>
  {/if}
</details>
