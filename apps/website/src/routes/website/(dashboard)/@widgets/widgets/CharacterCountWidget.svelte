<script lang="ts">
  import { getText, getTextBetween } from '@tiptap/core';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { comma } from '@typie/ui/utils';
  import { untrack } from 'svelte';
  import { textSerializers } from '@/pm/serializer';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
  import TypeIcon from '~icons/lucide/type';
  import Widget from '../Widget.svelte';
  import { getWidgetContext } from '../widget-context.svelte';
  import type { Editor } from '@tiptap/core';
  import type { Transaction } from '@tiptap/pm/state';

  type Props = {
    widgetId: string;
    data?: Record<string, unknown>;
  };

  let { widgetId, data = {} }: Props = $props();

  const widgetContext = getWidgetContext();
  const { editor } = $derived(widgetContext.env);
  let isCollapsed = $state((data.isCollapsed as boolean) ?? false);

  const toggleCollapse = () => {
    isCollapsed = !isCollapsed;
    widgetContext.updateWidget?.(widgetId, { ...data, isCollapsed });
  };

  let doc = $state('');
  let selection = $state('');

  const cleanDoc = $derived(doc.replaceAll('\u200B', ''));
  const cleanSelection = $derived(selection.replaceAll('\u200B', ''));

  const docCountWithWhitespace = $derived([...cleanDoc.replaceAll(/\s+/g, ' ').trim()].length);
  const docCountWithoutWhitespace = $derived([...cleanDoc.replaceAll(/\s/g, '').trim()].length);
  const docCountWithoutWhitespaceAndPunctuation = $derived([...cleanDoc.replaceAll(/[\s\p{P}]/gu, '').trim()].length);

  const selectionCountWithWhitespace = $derived([...cleanSelection.replaceAll(/\s+/g, ' ').trim()].length);
  const selectionCountWithoutWhitespace = $derived([...cleanSelection.replaceAll(/\s/g, '').trim()].length);
  const selectionCountWithoutWhitespaceAndPunctuation = $derived([...cleanSelection.replaceAll(/[\s\p{P}]/gu, '').trim()].length);

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
    if (editor) {
      return untrack(() => {
        editor.current.on('transaction', handler);

        doc = getText(editor.current.state.doc, {
          blockSeparator: '\n',
          textSerializers,
        });

        selection = getTextBetween(
          editor.current.state.doc,
          { from: editor.current.state.selection.from, to: editor.current.state.selection.to },
          {
            blockSeparator: '\n',
            textSerializers,
          },
        );

        return () => {
          editor?.current.off('transaction', handler);
        };
      });
    }
  });
</script>

<Widget collapsed={isCollapsed} icon={TypeIcon} title="글자 수">
  {#snippet headerActions()}
    <button
      class={flex({ alignItems: 'center', gap: '2px', color: 'text.subtle', cursor: 'pointer' })}
      onclick={toggleCollapse}
      type="button"
    >
      {#if isCollapsed}
        <span class={css({ fontSize: '13px', fontWeight: 'normal', color: 'text.subtle' })}>
          {comma(docCountWithWhitespace)}자
        </span>
      {/if}
      <Icon icon={isCollapsed ? ChevronDownIcon : ChevronUpIcon} size={14} />
    </button>
  {/snippet}

  <div class={flex({ flexDirection: 'column', gap: '8px' })}>
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
</Widget>
