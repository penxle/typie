<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { comma } from '@typie/ui/utils';
  import { fly } from 'svelte/transition';
  import IconChevronRight from '~icons/lucide/chevron-right';
  import IconType from '~icons/lucide/type';
  import type { Editor } from '$lib/editor/editor.svelte';

  type Props = {
    editor: Editor;
  };

  let { editor }: Props = $props();

  let open = $state(false);

  const counts = $derived(editor.characterCounts);
  const hasSelection = $derived(counts.selectionWithWhitespace > 0);
</script>

<details class={flex({ flexDirection: 'column', marginBottom: open ? '12px' : '8px' })} bind:open>
  <summary class={flex({ alignItems: 'center', gap: '4px', cursor: 'pointer', marginBottom: open ? '8px' : '0', userSelect: 'none' })}>
    <Icon style={{ color: 'text.faint' }} icon={IconType} size={12} />
    <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.subtle' })}>글자 수</div>
    <Icon style={css.raw({ color: 'text.faint', transform: open ? 'rotate(90deg)' : 'rotate(0deg)' })} icon={IconChevronRight} size={14} />
    <div class={css({ flexGrow: '1' })}></div>
    {#if !open}
      <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.subtle' })} in:fly={{ y: 2, duration: 150 }}>
        {#if hasSelection}
          {comma(counts.selectionWithWhitespace)}자 /
        {/if}
        {comma(counts.docWithWhitespace)}자
      </div>
    {/if}
  </summary>

  {#if open}
    <div class={flex({ flexDirection: 'column', gap: '2px' })} in:fly={{ y: -2, duration: 150 }}>
      <dl class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
        <dt class={css({ color: 'text.faint' })}>공백 포함</dt>
        <dd class={css({ fontWeight: 'medium', color: 'text.subtle' })}>
          {#if hasSelection}
            {comma(counts.selectionWithWhitespace)}자 /
          {/if}
          {comma(counts.docWithWhitespace)}자
        </dd>
      </dl>

      <dl class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
        <dt class={css({ color: 'text.faint' })}>공백 미포함</dt>
        <dd class={css({ fontWeight: 'medium', color: 'text.subtle' })}>
          {#if hasSelection}
            {comma(counts.selectionWithoutWhitespace)}자 /
          {/if}
          {comma(counts.docWithoutWhitespace)}자
        </dd>
      </dl>

      <dl class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
        <dt class={css({ color: 'text.faint' })}>공백/부호 미포함</dt>
        <dd class={css({ fontWeight: 'medium', color: 'text.subtle' })}>
          {#if hasSelection}
            {comma(counts.selectionWithoutWhitespaceAndPunctuation)}자 /
          {/if}
          {comma(counts.docWithoutWhitespaceAndPunctuation)}자
        </dd>
      </dl>
    </div>
  {/if}
</details>
