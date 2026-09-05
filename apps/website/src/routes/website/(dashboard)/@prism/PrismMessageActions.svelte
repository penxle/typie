<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import { onDestroy } from 'svelte';
  import CheckIcon from '~icons/lucide/check';
  import CopyIcon from '~icons/lucide/copy';
  import ThumbsDownIcon from '~icons/lucide/thumbs-down';
  import ThumbsUpIcon from '~icons/lucide/thumbs-up';
  import { expand } from './lib/motion.ts';
  import PrismReactionNote from './PrismReactionNote.svelte';
  import type { PrismRunMeta } from './prism-data.ts';

  type Props = {
    role: 'assistant' | 'user';
    text: string;
    at: number;
    persistent?: boolean;
    run?: PrismRunMeta;
    onReact?: (runId: string, reaction: 'UP' | 'DOWN' | null, note: string | null) => Promise<boolean>;
  };

  let { role, text, at, persistent = false, run, onReact }: Props = $props();

  const timeFormat = new Intl.DateTimeFormat('ko-KR', { hour: 'numeric', minute: '2-digit' });
  const sameYearFormat = new Intl.DateTimeFormat('ko-KR', {
    month: 'long',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
  });
  const otherYearFormat = new Intl.DateTimeFormat('ko-KR', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
  });
  const COPY_FEEDBACK_MS = 2000;

  const formatTimestamp = (value: number) => {
    const date = new Date(value);
    const today = new Date();
    const sameYear = date.getFullYear() === today.getFullYear();
    const sameDay = sameYear && date.getMonth() === today.getMonth() && date.getDate() === today.getDate();

    if (sameDay) return timeFormat.format(date);
    if (sameYear) return sameYearFormat.format(date);
    return otherYearFormat.format(date);
  };

  let copied = $state(false);
  let copyTimer: ReturnType<typeof setTimeout> | null = null;

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
      if (copyTimer !== null) clearTimeout(copyTimer);
      copyTimer = setTimeout(() => {
        copied = false;
        copyTimer = null;
      }, COPY_FEEDBACK_MS);
    } catch {
      Toast.error('복사하지 못했어요');
    }
  };

  onDestroy(() => {
    if (copyTimer !== null) clearTimeout(copyTimer);
  });

  let editing = $state(false);
  let note = $state('');
  let sending = $state(false);

  const send = async (reaction: 'UP' | 'DOWN' | null, reactionNote: string | null) => {
    if (!run || !onReact || sending) return false;
    sending = true;
    try {
      return await onReact(run.id, reaction, reactionNote);
    } finally {
      sending = false;
    }
  };

  const react = async (value: 'UP' | 'DOWN') => {
    if (!run) return;
    const clearing = run.reaction === value;
    const draft = note.trim();
    const kept = draft.length > 0 ? draft : run.reactionNote;
    const ok = clearing ? await send(null, null) : await send(value, kept);
    if (!ok) return;

    if (clearing) {
      note = '';
      editing = false;
    } else {
      editing = kept === null;
    }
  };

  const saveNote = async () => {
    if (!run?.reaction) return;
    if (await send(run.reaction, note.trim() || null)) editing = false;
  };

  const rowStyle = css({
    display: 'flex',
    alignItems: 'center',
    gap: '2px',
    marginTop: '6px',
    minHeight: '26px',
    color: 'text.hint',
    opacity: '0',
    transition: '[opacity 120ms ease]',
    '.group:hover &': { opacity: '100' },
    _groupFocusWithin: { opacity: '100' },
    '&[data-persistent]': { opacity: '100' },
  });
  const actionStyle = css({
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    size: '26px',
    borderRadius: '6px',
    color: 'text.muted',
    transition: '[color 120ms ease, background-color 120ms ease]',
    _hover: { color: 'text.default', backgroundColor: 'surface.hover' },
    _focusVisible: { color: 'text.default', outline: '2px solid token(colors.accent.default)', outlineOffset: '1px' },
    _pressed: { color: 'accent.default', backgroundColor: 'surface.active' },
    _disabled: { cursor: 'default', opacity: '40' },
  });
  const timeStyle = css({
    display: 'inline-flex',
    alignItems: 'center',
    height: '26px',
    paddingX: '4px',
    fontSize: '11px',
    lineHeight: '[1]',
    color: 'text.hint',
    whiteSpace: 'nowrap',
    opacity: '0',
    transition: '[opacity 120ms ease]',
    '.group:hover &': { opacity: '100' },
    _groupFocusWithin: { opacity: '100' },
  });
  const noteVisibleStyle = css.raw({
    gridTemplateRows: '[1fr]',
    marginTop: '6px',
    opacity: '100',
    pointerEvents: 'auto',
  });
  const noteStyle = css({
    display: 'grid',
    width: 'full',
    gridTemplateRows: '[0fr]',
    marginTop: '0',
    opacity: '0',
    pointerEvents: 'none',
    transition: '[grid-template-rows 160ms ease, margin-top 160ms ease, opacity 120ms ease]',
    '& > *': { minHeight: '0', overflow: 'hidden' },
    '.group:hover &': noteVisibleStyle,
    _groupFocusWithin: noteVisibleStyle,
    '&[data-persistent]': noteVisibleStyle,
  });
</script>

{#snippet timestamp()}
  <time class={timeStyle} data-message-time datetime={new Date(at).toISOString()}>{formatTimestamp(at)}</time>
{/snippet}

{#snippet copyButton()}
  <button class={actionStyle} aria-label={copied ? '복사됨' : '복사'} onclick={() => void copy()} type="button">
    <Icon icon={copied ? CheckIcon : CopyIcon} size={14} />
  </button>
{/snippet}

<div class={flex({ flexDirection: 'column', alignItems: role === 'user' ? 'flex-end' : 'flex-start' })}>
  <div class={rowStyle} data-message-actions data-persistent={persistent ? '' : undefined}>
    {#if role === 'user'}
      {@render timestamp()}
      {@render copyButton()}
    {:else}
      {@render copyButton()}
      <button
        class={actionStyle}
        aria-label="좋았어요"
        aria-pressed={run?.reaction === 'UP'}
        disabled={sending}
        onclick={() => void react('UP')}
        type="button"
      >
        <Icon icon={ThumbsUpIcon} size={14} />
      </button>
      <button
        class={actionStyle}
        aria-label="아쉬웠어요"
        aria-pressed={run?.reaction === 'DOWN'}
        disabled={sending}
        onclick={() => void react('DOWN')}
        type="button"
      >
        <Icon icon={ThumbsDownIcon} size={14} />
      </button>
      {@render timestamp()}
    {/if}
  </div>

  {#if role === 'assistant' && run?.reaction}
    <div class={css({ width: 'full', maxWidth: '360px' })} transition:expand>
      <div class={noteStyle} data-persistent={persistent ? '' : undefined} data-reaction-note-container>
        <div>
          <PrismReactionNote onSubmit={() => void saveNote()} savedNote={run.reactionNote} {sending} bind:editing bind:draft={note} />
        </div>
      </div>
    </div>
  {/if}
</div>
