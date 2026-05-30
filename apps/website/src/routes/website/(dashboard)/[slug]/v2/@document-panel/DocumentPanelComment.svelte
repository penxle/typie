<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import MessageSquareTextIcon from '~icons/lucide/message-square-text';
  import { getCommentContext } from '../@document-comments/context.svelte';
  import DocumentPanelCommentItem from './DocumentPanelCommentItem.svelte';
  import type { Editor } from '$lib/editor-ffi/editor.svelte';

  type Props = { editor: Editor | undefined };
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  let { editor: _editor }: Props = $props();

  const comments = getCommentContext();
  const list = $derived(comments ? (comments.showResolved ? comments.resolvedThreads : comments.threads) : []);
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
      {#if comments && list.length > 0}
        <div
          class={css({
            borderRadius: '4px',
            paddingX: '6px',
            paddingY: '2px',
            fontSize: '11px',
            fontWeight: 'semibold',
            color: 'accent.brand.default',
            backgroundColor: 'accent.brand.subtle',
          })}
        >
          {list.length}
        </div>
      {/if}
    </div>
    {#if comments}
      <button
        class={css({ fontSize: '12px', color: 'text.faint', transition: 'common', _hover: { color: 'text.subtle' } })}
        onclick={() => comments.setShowResolved(!comments.showResolved)}
        type="button"
      >
        {comments.showResolved ? '열린 코멘트' : '해결된 코멘트'}
      </button>
    {/if}
  </div>

  {#if !comments || list.length === 0}
    <div class={flex({ flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: '20px', paddingY: '60px' })}>
      <div class={center({ size: '64px', borderRadius: '16px', backgroundColor: 'surface.muted', color: 'text.faint' })}>
        <Icon icon={MessageSquareTextIcon} size={28} />
      </div>
      <p class={css({ fontSize: '13px', color: 'text.faint', textAlign: 'center' })}>
        {comments?.showResolved ? '해결된 코멘트가 없어요' : '아직 코멘트가 없어요'}
      </p>
    </div>
  {:else}
    <div class={flex({ flexDirection: 'column', overflowY: 'auto' })}>
      {#each list as thread (thread.id)}
        {#if comments.showResolved}
          <div class={css({ position: 'relative' })}>
            <DocumentPanelCommentItem active={false} thread$key={thread} />
            <button
              class={css({
                position: 'absolute',
                right: '12px',
                top: '10px',
                fontSize: '11px',
                color: 'accent.brand.default',
                _hover: { textDecoration: 'underline' },
              })}
              onclick={(e) => {
                e.stopPropagation();
                void comments.unresolveThread(thread.id);
              }}
              type="button"
            >
              다시 열기
            </button>
          </div>
        {:else}
          <DocumentPanelCommentItem
            active={comments.activeThreadId === thread.id}
            locatable={comments.isLocatable(thread.id)}
            onclick={() => comments.openFromPanel(thread.id)}
            thread$key={thread}
          />
        {/if}
      {/each}
    </div>
  {/if}
</div>
