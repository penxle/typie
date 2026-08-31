<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { autosize } from '@typie/ui/actions';
  import { Icon, RingSpinner } from '@typie/ui/components';
  import ArrowUpIcon from '~icons/lucide/arrow-up';

  type Props = {
    savedNote: string | null;
    sending: boolean;
    editing?: boolean;
    draft?: string;
    onSubmit: () => void;
  };

  let { savedNote, sending, onSubmit, editing = $bindable(false), draft = $bindable('') }: Props = $props();
  let input = $state<HTMLTextAreaElement>();

  $effect(() => {
    if (!editing) draft = savedNote ?? '';
  });

  $effect(() => {
    if (editing) input?.focus();
  });

  const draftStyle = flex({
    alignItems: 'flex-end',
    gap: '6px',
    width: 'full',
    minHeight: '34px',
    paddingLeft: '10px',
    paddingRight: '4px',
    paddingY: '4px',
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: '8px',
    backgroundColor: 'surface.subtle',
  });
  const savedStyle = flex({
    alignItems: 'center',
    gap: '6px',
    width: 'full',
    maxWidth: 'full',
    minHeight: '34px',
    paddingX: '8px',
    paddingY: '4px',
    borderRadius: '8px',
    backgroundColor: 'surface.muted',
  });
  const textStyle = css({
    flexGrow: '1',
    minWidth: '0',
    fontSize: '12px',
    lineHeight: '[1.5]',
    color: 'text.subtle',
    whiteSpace: 'pre-wrap',
  });
  const editStyle = css({
    flexShrink: '0',
    marginLeft: 'auto',
    fontSize: '11px',
    fontWeight: 'semibold',
    color: 'text.faint',
    _hover: { color: 'text.default' },
  });
  const submitStyle = flex({
    alignItems: 'center',
    justifyContent: 'center',
    flexShrink: '0',
    size: '22px',
    borderRadius: 'full',
    backgroundColor: 'surface.dark',
    color: 'text.bright',
  });
</script>

{#if !editing && savedNote}
  <div class={savedStyle} data-reaction-note data-reaction-note-state="saved">
    <p class={textStyle} data-reaction-note-text>{savedNote}</p>
    <button
      class={editStyle}
      aria-label="반응 메모 수정"
      onclick={() => {
        draft = savedNote;
        editing = true;
      }}
      type="button"
    >
      수정
    </button>
  </div>
{:else}
  <div class={draftStyle} data-reaction-note data-reaction-note-state="draft">
    <textarea
      bind:this={input}
      class={css({
        flexGrow: '1',
        minWidth: '0',
        maxHeight: '120px',
        paddingY: '3px',
        fontSize: '12px',
        lineHeight: '[1.5]',
        backgroundColor: 'transparent',
        resize: 'none',
        outline: 'none',
        _placeholder: { color: 'text.faint' },
      })}
      onkeydown={(event) => {
        if (event.key === 'Enter' && !event.shiftKey && !event.isComposing) {
          event.preventDefault();
          onSubmit();
        }
      }}
      placeholder="몇 자 덧붙이기"
      readonly={sending}
      rows={1}
      bind:value={draft}
      use:autosize={{ value: draft }}></textarea>
    <button
      class={submitStyle}
      aria-busy={sending}
      aria-label={sending ? '남기는 중' : '남기기'}
      disabled={sending}
      onclick={onSubmit}
      type="button"
    >
      {#if sending}
        <span class={flex({ alignItems: 'center', justifyContent: 'center', size: '10px' })} data-reaction-note-submit-spinner>
          <RingSpinner style={css.raw({ size: '10px' })} />
        </span>
      {:else}
        <Icon icon={ArrowUpIcon} size={10} />
      {/if}
    </button>
  </div>
{/if}
