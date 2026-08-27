<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, Menu, MenuItem, TimeAgo } from '@typie/ui/components';
  import dayjs from 'dayjs';
  import { tick } from 'svelte';
  import ArchiveIcon from '~icons/lucide/archive';
  import ArchiveRestoreIcon from '~icons/lucide/archive-restore';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import PencilIcon from '~icons/lucide/pencil';
  import SearchIcon from '~icons/lucide/search';
  import TrashIcon from '~icons/lucide/trash-2';
  import XIcon from '~icons/lucide/x';
  import { groupSessionsByRecency, hasUnread, matchesSessionQuery, sessionLabel } from './lib/session-groups.ts';

  type Session = {
    id: string;
    title?: string | null;
    archivedAt?: string | null;
    updatedAt: string;
    awaitingUser: boolean;
    unseenReviewCount: number;
  };

  type Props = {
    sessions: readonly Session[];
    currentId: string | null;
    onClose: () => void;
    onSelect: (id: string) => void;
    onArchive: (id: string) => Promise<void> | void;
    onUnarchive: (id: string) => Promise<void> | void;
    onRename: (id: string, title: string) => Promise<void> | void;
    onDelete: (session: Session) => void;
  };

  let { sessions, currentId, onClose, onSelect, onArchive, onUnarchive, onRename, onDelete }: Props = $props();

  let query = $state('');
  let archivedOpen = $state(false);
  let highlight = $state(-1);
  let editingId = $state<string | null>(null);
  let draft = $state('');
  let searchEl = $state<HTMLInputElement>();
  let editEl = $state<HTMLInputElement>();
  let listEl = $state<HTMLElement>();
  let pendingEdit: Session | null = null;

  const labelOf = sessionLabel;
  const unread = hasUnread;

  const searching = $derived(query.trim().length > 0);
  const matched = $derived(sessions.filter((session) => matchesSessionQuery(session.title, query)));
  const groups = $derived(groupSessionsByRecency(matched.filter((session) => session.archivedAt == null)));
  const archived = $derived(
    groupSessionsByRecency(matched.filter((session) => session.archivedAt != null)).flatMap((group) => group.sessions),
  );
  const showArchived = $derived(archived.length > 0 && (archivedOpen || searching));
  const visible = $derived([...groups.flatMap((group) => group.sessions), ...(showArchived ? archived : [])]);
  const indexOf = (id: string) => visible.findIndex((session) => session.id === id);

  $effect(() => {
    void query;
    highlight = -1;
  });

  $effect(() => {
    void tick().then(() => searchEl?.focus());
  });

  $effect(() => {
    if (highlight < 0) return;
    listEl?.querySelector(`[data-index="${highlight}"]`)?.scrollIntoView({ block: 'nearest' });
  });

  const move = (delta: number) => {
    if (visible.length === 0) return;
    highlight = Math.min(visible.length - 1, Math.max(0, highlight + delta));
  };

  const onKeydown = (event: KeyboardEvent) => {
    if (event.isComposing) return;
    const target = event.target as HTMLElement | null;
    if (editingId !== null && target === editEl) return;

    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      move(event.key === 'ArrowDown' ? 1 : -1);
    } else if (event.key === 'Enter') {
      if (target !== searchEl) return;
      event.preventDefault();
      const session = visible[Math.max(0, highlight)];
      if (session) onSelect(session.id);
    } else if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      onClose();
    } else if (target !== searchEl && event.key === '/') {
      event.preventDefault();
      searchEl?.focus();
    }
  };

  const startEdit = async (session: Session) => {
    editingId = session.id;
    draft = labelOf(session);
    await tick();
    editEl?.focus();
    editEl?.select();
  };

  const commitEdit = async () => {
    const id = editingId;
    if (id === null) return;
    editingId = null;
    const title = draft.trim();
    const session = sessions.find((candidate) => candidate.id === id);
    if (!session || title.length === 0 || title === labelOf(session)) return;
    await onRename(id, title);
  };

  const groupLabel = css({
    paddingX: '14px',
    paddingTop: '14px',
    paddingBottom: '4px',
    fontSize: '11px',
    fontWeight: 'medium',
    color: 'text.faint',
  });

  const row = css({
    position: 'relative',
    display: 'flex',
    alignItems: 'center',
    marginX: '6px',
    borderRadius: '6px',
    '&:hover, &[data-highlighted="true"]': { backgroundColor: 'surface.muted' },
    '&[data-current="true"] .title': { fontWeight: 'semibold', color: 'text.default' },
    '&:hover .more, &:focus-within .more, &[data-highlighted="true"] .more, & .more[data-open="true"]': { opacity: '100' },
  });

  const rowButton = css({
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
    flexGrow: '1',
    minWidth: '0',
    paddingLeft: '8px',
    paddingRight: '4px',
    paddingY: '7px',
    fontSize: '13px',
    color: 'text.subtle',
    textAlign: 'left',
  });

  const rowTitle = css({ flexGrow: '1', minWidth: '0', overflow: 'hidden', whiteSpace: 'nowrap', textOverflow: 'ellipsis' });

  const rowDot = css({ flexShrink: '0', size: '6px', borderRadius: 'full', backgroundColor: 'accent.info.default' });

  const more = css({
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    flexShrink: '0',
    marginRight: '4px',
    size: '24px',
    borderRadius: '4px',
    color: 'text.faint',
    opacity: '0',
    transition: '[opacity 120ms ease]',
    _hover: { color: 'text.subtle', backgroundColor: 'surface.subtle' },
  });

  const editInput = css({
    flexGrow: '1',
    minWidth: '0',
    marginX: '4px',
    marginY: '3px',
    paddingX: '4px',
    paddingY: '3px',
    borderWidth: '1px',
    borderColor: 'border.strong',
    borderRadius: '4px',
    fontSize: '13px',
    backgroundColor: 'surface.default',
    outline: 'none',
  });
</script>

<div
  class={flex({
    position: 'absolute',
    inset: '0',
    flexDirection: 'column',
    backgroundColor: 'surface.default',
    zIndex: '3',
    animation: '[rise-in 200ms cubic-bezier(0.23, 1, 0.32, 1) both]',
    _motionReduce: { animation: 'none' },
  })}
  aria-label="대화 목록"
  onkeydown={onKeydown}
  role="dialog"
  tabindex="-1"
>
  <div
    class={flex({
      alignItems: 'center',
      gap: '6px',
      paddingX: '10px',
      paddingY: '8px',
      borderBottomWidth: '1px',
      borderColor: 'border.subtle',
    })}
  >
    <div class={flex({ position: 'relative', alignItems: 'center', flexGrow: '1', minWidth: '0' })}>
      <Icon
        style={css.raw({ position: 'absolute', left: '8px', color: 'text.faint', pointerEvents: 'none' })}
        icon={SearchIcon}
        size={14}
      />
      <input
        bind:this={searchEl}
        class={css({
          width: 'full',
          paddingLeft: '28px',
          paddingRight: '8px',
          paddingY: '6px',
          borderRadius: '6px',
          fontSize: '13px',
          backgroundColor: 'surface.muted',
          outline: 'none',
          _placeholder: { color: 'text.faint' },
        })}
        placeholder="대화 검색"
        type="text"
        bind:value={query}
      />
    </div>
    <button
      class={css({
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        flexShrink: '0',
        size: '24px',
        borderRadius: '4px',
        color: 'text.faint',
        _hover: { color: 'text.subtle', backgroundColor: 'surface.muted' },
      })}
      aria-label="목록 닫기"
      onclick={onClose}
      type="button"
    >
      <Icon icon={XIcon} size={16} />
    </button>
  </div>

  <div
    bind:this={listEl}
    class={css({ flexGrow: '1', minHeight: '0', overflowY: 'auto', scrollbarWidth: 'none', paddingBottom: '12px' })}
    role="listbox"
  >
    {#if sessions.length === 0}
      <div class={css({ paddingX: '14px', paddingY: '24px', fontSize: '13px', color: 'text.faint', textAlign: 'center' })}>
        아직 대화가 없어요
      </div>
    {:else if visible.length === 0 && archived.length === 0}
      <div class={css({ paddingX: '14px', paddingY: '24px', fontSize: '13px', color: 'text.faint', textAlign: 'center' })}>
        "{query.trim()}"에 맞는 대화가 없어요
      </div>
    {/if}

    {#each groups as group (group.key)}
      <div class={groupLabel}>{group.label}</div>
      {#each group.sessions as session (session.id)}
        {@render item(session, indexOf(session.id), false)}
      {/each}
    {/each}

    {#if archived.length > 0}
      <button
        class={flex({
          alignItems: 'center',
          gap: '4px',
          width: 'full',
          paddingX: '14px',
          paddingTop: '14px',
          paddingBottom: '4px',
          fontSize: '11px',
          fontWeight: 'medium',
          color: 'text.faint',
          _hover: { color: 'text.subtle' },
        })}
        aria-expanded={showArchived}
        onclick={() => (archivedOpen = !archivedOpen)}
        type="button"
      >
        <Icon
          style={css.raw({ transition: '[transform 160ms ease]', transform: showArchived ? 'rotate(90deg)' : 'rotate(0deg)' })}
          icon={ChevronRightIcon}
          size={12}
        />
        보관됨 ({archived.length})
      </button>
      {#if showArchived}
        {#each archived as session (session.id)}
          {@render item(session, indexOf(session.id), true)}
        {/each}
      {/if}
    {/if}
  </div>
</div>

{#snippet item(session: Session, index: number, isArchived: boolean)}
  <div
    class={row}
    aria-selected={index === highlight}
    data-current={session.id === currentId}
    data-highlighted={index === highlight}
    data-index={index}
    role="option"
  >
    {#if editingId === session.id}
      <input
        bind:this={editEl}
        class={editInput}
        aria-label="대화 제목"
        maxlength="100"
        onblur={() => void commitEdit()}
        onkeydown={(event) => {
          if (event.isComposing) return;
          if (event.key === 'Enter') {
            event.preventDefault();
            void commitEdit();
          } else if (event.key === 'Escape') {
            event.preventDefault();
            event.stopPropagation();
            editingId = null;
          }
        }}
        type="text"
        bind:value={draft}
      />
    {:else}
      <button class={rowButton} onclick={() => onSelect(session.id)} type="button">
        <span class={`title ${rowTitle}`}>{labelOf(session)}</span>
        {#if unread(session)}
          <span class={rowDot} aria-hidden="true"></span>
          <span class={css({ srOnly: true })}>새 상태 변화 있음</span>
        {/if}
        <TimeAgo
          style={css.raw({ flexShrink: '0', fontSize: '11px', color: 'text.faint' })}
          timestamp={dayjs(session.updatedAt).valueOf()}
        />
      </button>

      <Menu
        ontransitionend={() => {
          const target = pendingEdit;
          pendingEdit = null;
          if (target) void startEdit(target);
        }}
        placement="bottom-end"
      >
        {#snippet button({ open })}
          <div class={`more ${more}`} aria-label="대화 메뉴" data-open={open}>
            <Icon icon={EllipsisIcon} size={14} />
          </div>
        {/snippet}

        {#snippet children({ close })}
          <MenuItem
            icon={PencilIcon}
            onclick={() => {
              pendingEdit = session;
              close();
            }}
          >
            이름 바꾸기
          </MenuItem>
          {#if isArchived}
            <MenuItem
              icon={ArchiveRestoreIcon}
              onclick={() => {
                close();
                void onUnarchive(session.id);
              }}
            >
              복원
            </MenuItem>
          {:else}
            <MenuItem
              icon={ArchiveIcon}
              onclick={() => {
                close();
                void onArchive(session.id);
              }}
            >
              보관
            </MenuItem>
          {/if}
          <MenuItem
            icon={TrashIcon}
            onclick={() => {
              close();
              onDelete(session);
            }}
            variant="danger"
          >
            삭제
          </MenuItem>
        {/snippet}
      </Menu>
    {/if}
  </div>
{/snippet}
