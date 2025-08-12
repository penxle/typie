<script lang="ts">
  import { getText, getTextBetween } from '@tiptap/core';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { comma } from '@typie/ui/utils';
  import { fly } from 'svelte/transition';
  import { textSerializers } from '@/pm/serializer';
  import IconChevronRight from '~icons/lucide/chevron-right';
  import IconType from '~icons/lucide/type';
  import type { Editor } from '@tiptap/core';
  import type { Transaction } from '@tiptap/pm/state';
  import type { Ref } from '@typie/ui/utils';

  type Props = {
    editor?: Ref<Editor>;
  };

  let { editor }: Props = $props();

  let open = $state(false);

  let doc = $state('');
  let selection = $state('');

  const docCountWithWhitespace = $derived([...doc.replaceAll(/\s+/g, ' ').trim()].length);
  const docCountWithoutWhitespace = $derived([...doc.replaceAll(/\s/g, '').trim()].length);
  const docCountWithoutWhitespaceAndPunctuation = $derived([...doc.replaceAll(/[\s\p{P}]/gu, '').trim()].length);

  const selectionCountWithWhitespace = $derived([...selection.replaceAll(/\s+/g, ' ').trim()].length);
  const selectionCountWithoutWhitespace = $derived([...selection.replaceAll(/\s/g, '').trim()].length);
  const selectionCountWithoutWhitespaceAndPunctuation = $derived([...selection.replaceAll(/[\s\p{P}]/gu, '').trim()].length);

  const handler = ({ editor, transaction }: { editor: Editor; transaction: Transaction }) => {
    if (transaction.docChanged) {
      doc = getText(editor.state.doc, {
        blockSeparator: '\n',
        textSerializers,
      });
    }

    if (transaction.selectionSet) {
      selection = getTextBetween(
        editor.state.doc,
        { from: editor.state.selection.from, to: editor.state.selection.to },
        {
          blockSeparator: '\n',
          textSerializers,
        },
      );
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
        {#if selection}
          {comma(selectionCountWithWhitespace)}자 /
        {/if}
        {comma(docCountWithWhitespace)}자
      </div>
    {/if}
  </summary>

  {#if open}
    <div class={flex({ flexDirection: 'column', gap: '2px' })} in:fly={{ y: -2, duration: 150 }}>
      <dl class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
        <dt class={css({ color: 'text.faint' })}>공백 포함</dt>
        <dd class={css({ fontWeight: 'medium', color: 'text.subtle' })}>
          {#if selection}
            {comma(selectionCountWithWhitespace)}자 /
          {/if}
          {comma(docCountWithWhitespace)}자
        </dd>
      </dl>

      <dl class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
        <dt class={css({ color: 'text.faint' })}>공백 미포함</dt>
        <dd class={css({ fontWeight: 'medium', color: 'text.subtle' })}>
          {#if selection}
            {comma(selectionCountWithoutWhitespace)}자 /
          {/if}
          {comma(docCountWithoutWhitespace)}자
        </dd>
      </dl>

      <dl class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
        <dt class={css({ color: 'text.faint' })}>공백/부호 미포함</dt>
        <dd class={css({ fontWeight: 'medium', color: 'text.subtle' })}>
          {#if selection}
            {comma(selectionCountWithoutWhitespaceAndPunctuation)}자 /
          {/if}
          {comma(docCountWithoutWhitespaceAndPunctuation)}자
        </dd>
      </dl>
    </div>
  {/if}
</details>
