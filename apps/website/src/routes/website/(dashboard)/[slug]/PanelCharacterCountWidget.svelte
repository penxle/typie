<script lang="ts">
  import { getText, getTextBetween } from '@tiptap/core';
  import { loadModule } from 'cld3-asm';
  import stringWidth from 'string-width';
  import { fly } from 'svelte/transition';
  import { textSerializers } from '@/pm/serializer';
  import IconChevronRight from '~icons/lucide/chevron-right';
  import IconInfo from '~icons/lucide/info';
  import IconType from '~icons/lucide/type';
  import { Icon, Tooltip } from '$lib/components';
  import { comma, debounce } from '$lib/utils';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import type { Editor } from '@tiptap/core';
  import type { Transaction } from '@tiptap/pm/state';
  import type { CldFactory } from 'cld3-asm';
  import type { Ref } from '$lib/utils';

  type Props = {
    editor?: Ref<Editor>;
  };

  let { editor }: Props = $props();

  let open = $state(false);

  let doc = $state('');
  let selection = $state('');

  let isCJK = $state(
    typeof window !== 'undefined' && navigator?.language
      ? ['ko', 'ja', 'zh'].some((lang) => navigator.language.toLowerCase().startsWith(lang))
      : true, // NOTE: CJK를 기본값으로 함
  );
  let cld3Factory: CldFactory | null = null;

  const docCountWithWhitespace = $derived([...doc.replaceAll(/\s+/g, ' ').trim()].length);
  const docCountWithoutWhitespace = $derived([...doc.replaceAll(/\s/g, '').trim()].length);
  const docCountWithoutWhitespaceAndPunctuation = $derived([...doc.replaceAll(/[\s\p{P}]/gu, '').trim()].length);
  const docVisualWidth = $derived(stringWidth(doc, { ambiguousIsNarrow: !isCJK }));

  const selectionCountWithWhitespace = $derived([...selection.replaceAll(/\s+/g, ' ').trim()].length);
  const selectionCountWithoutWhitespace = $derived([...selection.replaceAll(/\s/g, '').trim()].length);
  const selectionCountWithoutWhitespaceAndPunctuation = $derived([...selection.replaceAll(/[\s\p{P}]/gu, '').trim()].length);
  const selectionVisualWidth = $derived(stringWidth(selection, { ambiguousIsNarrow: !isCJK }));

  const detectLanguage = debounce(async (text: string) => {
    try {
      if (!cld3Factory) {
        cld3Factory = await loadModule();
      }

      const sampleLength = Math.min(text.length, 1000);
      const identifier = cld3Factory.create(0, sampleLength);
      const result = identifier.findLanguage(text.slice(0, sampleLength));

      if (result) {
        const lang = result.language;
        isCJK = ['ko', 'zh', 'ja'].includes(lang);
      }

      identifier.dispose();
    } catch {
      // ignore
    }
  }, 500);

  $effect(() => {
    if (doc) {
      detectLanguage(doc);
    }
  });

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

      <dl class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
        <dt class={flex({ alignItems: 'center', gap: '4px', color: 'text.faint' })}>
          전각 기준
          <Tooltip
            message="한글, 한자 등은 1자, 영어와 숫자는 0.5자로 계산됩니다. 일부 특수기호는 환경에 따라 다르게 계산됩니다. (현재 로케일: {isCJK
              ? 'CJK'
              : 'non-CJK'})"
            placement="bottom-end"
          >
            <Icon style={css.raw({ color: 'text.faint' })} icon={IconInfo} size={12} />
          </Tooltip>
        </dt>
        <dd class={css({ fontWeight: 'medium', color: 'text.subtle' })}>
          {#if selection}
            {comma(selectionVisualWidth / 2)}자 /
          {/if}
          {comma(docVisualWidth / 2)}자
        </dd>
      </dl>
    </div>
  {/if}
</details>
