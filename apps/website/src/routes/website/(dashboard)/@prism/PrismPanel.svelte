<script lang="ts">
  import { createFragment, createMutation, createSubscription } from '@mearie/svelte';
  import { TypieError } from '@typie/lib/errors';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { pointerCapture } from '@typie/ui/actions';
  import { Icon, Menu, MenuItem } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Dialog, Toast } from '@typie/ui/notification';
  import { clamp } from '@typie/ui/utils';
  import { tick, untrack } from 'svelte';
  import ArchiveIcon from '~icons/lucide/archive';
  import ArchiveRestoreIcon from '~icons/lucide/archive-restore';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import HistoryIcon from '~icons/lucide/history';
  import PencilIcon from '~icons/lucide/pencil';
  import PlusIcon from '~icons/lucide/plus';
  import PyramidIcon from '~icons/lucide/pyramid';
  import TrashIcon from '~icons/lucide/trash-2';
  import XIcon from '~icons/lucide/x';
  import { cache } from '$lib/graphql';
  import { unwrapError } from '$lib/graphql/error';
  import { graphql } from '$mearie';
  import { sessionLabel } from './lib/session-groups.ts';
  import { createPrismChat } from './prism-chat.svelte';
  import { fetchSessionLog, toFrame } from './prism-data';
  import { PRISM_PANEL_MAX, PRISM_PANEL_MIN } from './prism-panel.ts';
  import { createPrismSessionState } from './prism-session.svelte';
  import PrismComposer from './PrismComposer.svelte';
  import PrismGateCard from './PrismGateCard.svelte';
  import PrismSessionList from './PrismSessionList.svelte';
  import PrismTranscript from './PrismTranscript.svelte';
  import type { DashboardLayout_PrismPanel_user$key } from '$mearie';

  type Props = {
    user$key: DashboardLayout_PrismPanel_user$key;
  };

  let { user$key }: Props = $props();

  const app = getAppContext();

  const user = createFragment(
    graphql(`
      fragment DashboardLayout_PrismPanel_user on User {
        id
        preferences

        prismSessions(includeArchived: true) {
          id
          title
          archivedAt
          updatedAt
        }
      }
    `),
    () => user$key,
  );

  const [archivePrismSession] = createMutation(
    graphql(`
      mutation DashboardLayout_PrismPanel_Archive_Mutation($input: ArchivePrismSessionInput!) {
        archivePrismSession(input: $input) {
          id
          archivedAt
        }
      }
    `),
  );

  const [unarchivePrismSession] = createMutation(
    graphql(`
      mutation DashboardLayout_PrismPanel_Unarchive_Mutation($input: UnarchivePrismSessionInput!) {
        unarchivePrismSession(input: $input) {
          id
          archivedAt
        }
      }
    `),
  );

  const [renamePrismSession] = createMutation(
    graphql(`
      mutation DashboardLayout_PrismPanel_Rename_Mutation($input: RenamePrismSessionInput!) {
        renamePrismSession(input: $input) {
          id
          title
        }
      }
    `),
  );

  const [deletePrismSession] = createMutation(
    graphql(`
      mutation DashboardLayout_PrismPanel_Delete_Mutation($input: DeletePrismSessionInput!) {
        deletePrismSession(input: $input) {
          id
        }
      }
    `),
  );

  const [sendPrismMessage] = createMutation(
    graphql(`
      mutation DashboardLayout_PrismPanel_Send_Mutation($input: SendPrismMessageInput!) {
        sendPrismMessage(input: $input) {
          runSeq

          session {
            id
            title
            archivedAt
            updatedAt
          }
        }
      }
    `),
  );

  const [cancelPrismRun] = createMutation(
    graphql(`
      mutation DashboardLayout_PrismPanel_Cancel_Mutation($input: CancelPrismRunInput!) {
        cancelPrismRun(input: $input) {
          id
        }
      }
    `),
  );

  const selected = createPrismSessionState(user.data.id);
  const sessions = $derived(user.data.prismSessions);

  const chat = createPrismChat({
    loadLog: fetchSessionLog,
    send: async (sessionId, message) => {
      const resp = await sendPrismMessage({ input: { sessionId: sessionId ?? undefined, message } });
      return { sessionId: resp.sendPrismMessage.session.id, runSeq: resp.sendPrismMessage.runSeq };
    },
    cancel: async (sessionId) => {
      await cancelPrismRun({ input: { sessionId } });
    },
  });

  let composer = $state<PrismComposer>();
  let listOpen = $state(false);
  const currentSession = $derived(sessions.find((session) => session.id === selected.current) ?? null);
  const currentTitle = $derived(currentSession ? sessionLabel(currentSession) : '새 대화');

  // 세션 액션은 헤더(현재 세션)와 목록(임의 세션) 두 진입점이 같은 것을 공유한다
  // 낙관적 갱신(mearie optimisticResponse) — 응답 전까지 목록·헤더가 이전 값으로 되돌아가는 깜빡임을 없앤다
  const archiveSession = async (id: string) => {
    try {
      await archivePrismSession(
        { input: { sessionId: id } },
        { metadata: { cache: { optimisticResponse: { archivePrismSession: { id, archivedAt: new Date().toISOString() } } } } },
      );
    } catch {
      Toast.error('보관하지 못했어요. 잠시 후 다시 시도해 주세요');
      return;
    }

    if (selected.current === id) {
      selected.current = null;
    }
  };

  const unarchiveSession = async (id: string) => {
    try {
      await unarchivePrismSession(
        { input: { sessionId: id } },
        { metadata: { cache: { optimisticResponse: { unarchivePrismSession: { id, archivedAt: null } } } } },
      );
    } catch {
      Toast.error('복원하지 못했어요. 잠시 후 다시 시도해 주세요');
    }
  };

  const renameSession = async (id: string, title: string) => {
    try {
      await renamePrismSession(
        { input: { sessionId: id, title } },
        { metadata: { cache: { optimisticResponse: { renamePrismSession: { id, title } } } } },
      );
    } catch {
      Toast.error('이름을 바꾸지 못했어요. 잠시 후 다시 시도해 주세요');
    }
  };

  const requestDelete = (session: { id: string; title?: string | null }) => {
    Dialog.confirm({
      title: '대화를 삭제하시겠어요?',
      message: `"${sessionLabel(session)}" 대화가 목록에서 사라지고 되돌릴 수 없어요.`,
      action: 'danger',
      actionLabel: '삭제',
      actionHandler: async () => {
        try {
          await deletePrismSession({ input: { sessionId: session.id } });
        } catch {
          Toast.error('삭제하지 못했어요. 잠시 후 다시 시도해 주세요');
          return;
        }

        if (selected.current === session.id) {
          selected.current = null;
        }

        cache.invalidate({ __typename: 'User', id: user.data.id, $field: 'prismSessions' });
      },
    });
  };

  // 헤더 제목 인라인 편집 — 목록의 편집과 같은 키 규칙(Enter 확정·Esc 취소·blur 확정)
  let titleEditing = $state(false);
  let titleDraft = $state('');
  let titleInput = $state<HTMLInputElement>();
  // 메뉴는 닫히며 포커스를 트리거로 되돌린다(focus-trap returnFocus) — 바로 입력을 띄우면 그 복귀가 blur 확정을
  // 발동시켜 즉시 닫힌다. 편집은 메뉴의 닫힘 전환이 끝난 뒤(ontransitionend) 시작한다.
  let pendingTitleEdit = false;

  const startTitleEdit = async () => {
    if (!currentSession) return;
    titleDraft = sessionLabel(currentSession);
    titleEditing = true;
    await tick();
    titleInput?.focus();
    titleInput?.select();
  };

  const commitTitleEdit = async () => {
    if (!titleEditing) return;
    titleEditing = false;
    const session = currentSession;
    const title = titleDraft.trim();
    if (!session || title.length === 0 || title === sessionLabel(session)) return;
    await renameSession(session.id, title);
  };

  $effect(() => {
    void selected.current;
    titleEditing = false;
  });

  const headerButton = css.raw({
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    flexShrink: '0',
    borderRadius: '4px',
    size: '24px',
    color: 'text.faint',
    transition: '[transform 160ms cubic-bezier(0.23, 1, 0.32, 1)]',
    _hover: { color: 'text.subtle', backgroundColor: 'surface.muted' },
    _active: { transform: 'scale(0.95)' },
  });
  let prevPanelOpen: boolean | null = null;

  $effect(() => {
    const open = app.state.prismAccess && app.preference.current.prismPanelOpen;
    const transition = prevPanelOpen === false && open;
    prevPanelOpen = open;
    if (transition) {
      composer?.focus();
    }
  });

  $effect(() => {
    if (!app.state.prismAccess || !app.preference.current.prismPanelOpen) {
      return;
    }

    // load는 chat 내부 $state(generation·sessionId 등)를 읽고 쓰므로 추적 밖에서 부른다 — 추적하면
    // generation 증가가 이 effect를 다시 깨워 새 세션(id null)에서 무한 루프가 된다.
    const id = selected.current;
    untrack(() => void chat.load(id));
  });

  let betaGate = $state<'prism_beta_required' | null>(null);
  const aiOptIn = $derived((user.data.preferences.aiOptIn as boolean | undefined) ?? false);
  const gate = $derived(betaGate ?? (app.state.subscribed ? (aiOptIn ? null : 'ai_opt_in_required') : 'subscription_required'));

  createSubscription(
    graphql(`
      subscription DashboardLayout_PrismPanel_Events_Subscription($sessionId: ID!, $cursor: Int) {
        prismSessionEvents(cursor: $cursor, sessionId: $sessionId)
      }
    `),
    () => ({ sessionId: chat.sessionId ?? '', cursor: chat.seedCursor }),
    () => ({
      skip: chat.sessionId === null || !app.state.prismAccess || !app.preference.current.prismPanelOpen || chat.loading,
      onData: (data) => {
        chat.receive(toFrame(data.prismSessionEvents));
      },
      onError: (err) => {
        const error = unwrapError(err);
        if (error instanceof TypieError && error.code === 'prism_beta_required') {
          betaGate = 'prism_beta_required';
          return;
        }

        chat.error = '실시간 연결이 끊겼어요. 패널을 닫았다 다시 열면 이어져요';
      },
    }),
  );

  const onSend = async (text: string) => {
    const creating = chat.sessionId === null;

    try {
      const result = await chat.send(text);
      selected.current = result.sessionId;

      if (creating) {
        cache.invalidate({ __typename: 'User', id: user.data.id, $field: 'prismSessions' });
      }
    } catch (err) {
      const error = unwrapError(err);
      const code = error instanceof TypieError ? error.code : null;

      if (code === 'prism_beta_required') {
        betaGate = 'prism_beta_required';
      } else if (code === 'prism_run_active') {
        Toast.error('답변이 끝난 뒤에 보낼 수 있어요');
      } else {
        Toast.error('메시지를 보내지 못했어요. 잠시 후 다시 시도해 주세요');
      }

      throw err;
    }
  };

  type ResizeSession = {
    startX: number;
    startWidth: number;
  };

  let previewWidth = $state<number | null>(null);
  const width = $derived(clamp(previewWidth ?? app.preference.current.prismPanelWidth, PRISM_PANEL_MIN, PRISM_PANEL_MAX));

  const updateResize = (session: ResizeSession, event: PointerEvent) => {
    previewWidth = clamp(session.startWidth + Math.round(session.startX - event.clientX), PRISM_PANEL_MIN, PRISM_PANEL_MAX);
  };
</script>

{#if app.state.prismAccess}
  <aside
    style:--min-width={`${PRISM_PANEL_MIN}px`}
    style:--width={`${width}px`}
    style:--max-width={`${PRISM_PANEL_MAX}px`}
    class={flex({
      position: 'relative',
      flexDirection: 'column',
      flexShrink: '0',
      height: 'full',
      minWidth: app.preference.current.prismPanelOpen ? 'var(--min-width)' : '0',
      width: app.preference.current.prismPanelOpen ? 'var(--width)' : '0',
      maxWidth: app.preference.current.prismPanelOpen ? 'var(--max-width)' : '0',
      opacity: app.preference.current.prismPanelOpen ? '100' : '0',
      transitionProperty: '[min-width, max-width, opacity]',
      transitionDuration: '200ms',
      transitionTimingFunction: 'ease',
      willChange: 'min-width, max-width, opacity',
      overflow: 'hidden',
      borderLeftWidth: '1px',
      borderColor: 'border.subtle',
      backgroundColor: 'surface.default',
      zIndex: 'panel',
    })}
  >
    <div
      class={css({
        position: 'absolute',
        top: '0',
        bottom: '0',
        left: '0',
        width: '8px',
        cursor: 'col-resize',
        zIndex: '1',
        _hoverAfter: {
          content: '""',
          display: 'block',
          borderRightRadius: '4px',
          height: 'full',
          width: '2px',
          backgroundColor: 'border.strong',
          opacity: '50',
        },
      })}
      use:pointerCapture={{
        start: (event): ResizeSession | null => {
          if (!event.isPrimary || event.button !== 0) return null;
          event.preventDefault();
          const session = { startX: event.clientX, startWidth: width };
          previewWidth = session.startWidth;
          return session;
        },
        move: updateResize,
        end: (session, event) => {
          updateResize(session, event);
          if (previewWidth !== null) app.preference.current.prismPanelWidth = previewWidth;
          previewWidth = null;
        },
        cancel: () => {
          previewWidth = null;
        },
      }}
    ></div>

    <header
      class={flex({
        alignItems: 'center',
        gap: '8px',
        height: '44px',
        paddingX: '14px',
        borderBottomWidth: '1px',
        borderColor: 'border.subtle',
        flexShrink: '0',
      })}
    >
      <Icon style={css.raw({ color: 'text.subtle', flexShrink: '0' })} icon={PyramidIcon} size={16} />
      <span class={css({ fontSize: '13px', fontWeight: 'semibold', letterSpacing: '[0.04em]' })}>PRISM</span>

      {#if titleEditing}
        <input
          bind:this={titleInput}
          class={css({
            marginLeft: 'auto',
            minWidth: '0',
            maxWidth: '220px',
            width: 'full',
            paddingX: '8px',
            paddingY: '3px',
            borderWidth: '1px',
            borderColor: 'border.strong',
            borderRadius: '6px',
            fontSize: '12px',
            backgroundColor: 'surface.default',
            outline: 'none',
          })}
          aria-label="대화 제목"
          maxlength="100"
          onblur={() => void commitTitleEdit()}
          onkeydown={(event) => {
            if (event.isComposing) return;
            if (event.key === 'Enter') {
              event.preventDefault();
              void commitTitleEdit();
            } else if (event.key === 'Escape') {
              event.preventDefault();
              event.stopPropagation();
              titleEditing = false;
            }
          }}
          type="text"
          bind:value={titleDraft}
        />
      {:else}
        <button
          class={css({
            marginLeft: 'auto',
            minWidth: '0',
            maxWidth: '220px',
            paddingX: '8px',
            paddingY: '4px',
            borderRadius: '6px',
            fontSize: '12px',
            color: 'text.subtle',
            overflow: 'hidden',
            whiteSpace: 'nowrap',
            textOverflow: 'ellipsis',
            backgroundColor: listOpen ? 'surface.muted' : 'transparent',
            _hover: { backgroundColor: 'surface.muted' },
          })}
          onclick={() => (listOpen = !listOpen)}
          title={currentTitle}
          type="button"
        >
          {currentTitle}
        </button>
      {/if}

      {#if currentSession}
        {@const session = currentSession}
        <Menu
          ontransitionend={() => {
            if (!pendingTitleEdit) return;
            pendingTitleEdit = false;
            void startTitleEdit();
          }}
          placement="bottom-end"
        >
          {#snippet button({ open })}
            <div class={css(headerButton, open ? { color: 'text.default', backgroundColor: 'surface.muted' } : {})} aria-label="대화 메뉴">
              <Icon icon={EllipsisIcon} size={16} />
            </div>
          {/snippet}

          {#snippet children({ close })}
            <MenuItem
              icon={PencilIcon}
              onclick={() => {
                pendingTitleEdit = true;
                close();
              }}
            >
              이름 바꾸기
            </MenuItem>
            {#if session.archivedAt == null}
              <MenuItem
                icon={ArchiveIcon}
                onclick={() => {
                  close();
                  void archiveSession(session.id);
                }}
              >
                보관
              </MenuItem>
            {:else}
              <MenuItem
                icon={ArchiveRestoreIcon}
                onclick={() => {
                  close();
                  void unarchiveSession(session.id);
                }}
              >
                복원
              </MenuItem>
            {/if}
            <MenuItem
              icon={TrashIcon}
              onclick={() => {
                close();
                requestDelete(session);
              }}
              variant="danger"
            >
              삭제
            </MenuItem>
          {/snippet}
        </Menu>
      {/if}

      <button
        class={css(headerButton)}
        aria-label="새 대화"
        onclick={() => {
          selected.current = null;
          listOpen = false;
          composer?.focus();
        }}
        type="button"
      >
        <Icon icon={PlusIcon} size={16} />
      </button>

      <button
        class={css(headerButton, listOpen ? { color: 'text.default', backgroundColor: 'surface.muted' } : {})}
        aria-label="대화 목록"
        aria-pressed={listOpen}
        onclick={() => (listOpen = !listOpen)}
        type="button"
      >
        <Icon icon={HistoryIcon} size={16} />
      </button>

      <button
        class={css(headerButton)}
        aria-label="AI 패널 닫기"
        onclick={() => (app.preference.current.prismPanelOpen = false)}
        type="button"
      >
        <Icon icon={XIcon} size={16} />
      </button>
    </header>

    <div class={flex({ position: 'relative', flexDirection: 'column', flexGrow: '1', minHeight: '0' })}>
      {#if gate}
        <PrismGateCard reason={gate} />
      {:else}
        {#if listOpen}
          <PrismSessionList
            currentId={selected.current}
            onArchive={archiveSession}
            onClose={() => (listOpen = false)}
            onDelete={requestDelete}
            onRename={renameSession}
            onSelect={(id) => {
              selected.current = id;
              listOpen = false;
            }}
            onUnarchive={unarchiveSession}
            {sessions}
          />
        {/if}

        {#key chat.generation}
          <PrismTranscript loading={chat.loading} pending={chat.pending} transcript={chat.transcript} />
        {/key}

        {#if chat.error}
          <div class={css({ paddingX: '14px', paddingBottom: '8px', fontSize: '11px', color: 'text.danger' })}>{chat.error}</div>
        {/if}

        <PrismComposer
          bind:this={composer}
          disabled={false}
          {onSend}
          onStop={async () => {
            try {
              await chat.stop();
            } catch {
              Toast.error('중단하지 못했어요. 잠시 후 다시 시도해 주세요');
            }
          }}
          running={chat.transcript.run === 'running'}
        />
      {/if}
    </div>
  </aside>
{/if}
