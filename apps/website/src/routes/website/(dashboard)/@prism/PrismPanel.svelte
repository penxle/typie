<script lang="ts">
  import { createFragment, createMutation, createSubscription } from '@mearie/svelte';
  import * as Sentry from '@sentry/sveltekit';
  import { TypieError } from '@typie/lib/errors';
  import { effectiveResolver, pendingRootRequests, runningWorkflows } from '@typie/prism';
  import { css, cx } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, Icon, Menu, MenuItem } from '@typie/ui/components';
  import { getAppContext, getThemeContext } from '@typie/ui/context';
  import { Dialog, Toast } from '@typie/ui/notification';
  import { prefersReducedMotion } from '@typie/ui/state';
  import mixpanel from 'mixpanel-browser';
  import { tick, untrack } from 'svelte';
  import ArchiveIcon from '~icons/lucide/archive';
  import ArchiveRestoreIcon from '~icons/lucide/archive-restore';
  import CircleAlertIcon from '~icons/lucide/circle-alert';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import HistoryIcon from '~icons/lucide/history';
  import PencilIcon from '~icons/lucide/pencil';
  import PlusIcon from '~icons/lucide/plus';
  import TrashIcon from '~icons/lucide/trash-2';
  import { page } from '$app/state';
  import { cache, wsStatus } from '$lib/graphql';
  import { unwrapError } from '$lib/graphql/error';
  import { getOpenDocuments } from '$lib/prism/open-documents.svelte';
  import { takeSessionJump } from '$lib/prism/session-jump.svelte';
  import PrismSpinner from '$lib/prism-ui/PrismSpinner.svelte';
  import { graphql } from '$mearie';
  import { AutoResolver } from './lib/auto-resolve.svelte.ts';
  import { backoffDelay } from './lib/backoff.ts';
  import { commandGate, commandNameOf } from './lib/commands.ts';
  import { expand, rise, swap } from './lib/motion.ts';
  import { hasUnread, sessionLabel } from './lib/session-groups.ts';
  import { prismAccessUnavailableMessage } from './prism-access.ts';
  import { createPrismChat } from './prism-chat.svelte';
  import { fetchTranscript, toFrame } from './prism-data';
  import { createPrismSessionState } from './prism-session.svelte';
  import PrismBadgeDot from './PrismBadgeDot.svelte';
  import PrismComposer from './PrismComposer.svelte';
  import PrismGateCard from './PrismGateCard.svelte';
  import PrismPanelHeader from './PrismPanelHeader.svelte';
  import PrismPanelIndicator from './PrismPanelIndicator.svelte';
  import PrismPushCard from './PrismPushCard.svelte';
  import PrismSessionList from './PrismSessionList.svelte';
  import PrismTranscript from './PrismTranscript.svelte';
  import { startChipsFor } from './start-chips.ts';
  import { clientResolvers } from './tools/index.ts';
  import { workflowApps } from './workflows/index.ts';
  import type { ToolPolicy, WorkflowMessage } from '@typie/prism';
  import type { DashboardLayout_PrismPanel_user$key } from '$mearie';
  import type { PrismAccessReason } from './prism-access.ts';
  import type { StartChip } from './start-chips.ts';

  type Props = {
    user$key: DashboardLayout_PrismPanel_user$key;
  };

  let { user$key }: Props = $props();

  const app = getAppContext();
  const theme = getThemeContext();

  $effect(() => {
    void theme.currentThemeVariant;
    window.dispatchEvent(new Event('typie-prism-themechange'));
  });

  const user = createFragment(
    graphql(`
      fragment DashboardLayout_PrismPanel_user on User {
        id
        entitled
        preferences
        prismAccess

        prismCredit {
          balance
        }

        sites {
          id
        }

        prismCommands {
          name
          description
          argumentHint
        }

        prismSessions(includeArchived: true) {
          id
          title
          archivedAt
          updatedAt
          toolPolicy
          awaitingUser
          unseenReviewCount
        }
      }
    `),
    () => user$key,
  );
  const aiOptIn = $derived((user.data.preferences.aiOptIn as boolean | undefined) ?? false);
  let betaGate = $state<'prism_beta_required' | null>(null);
  const accessReason: PrismAccessReason | null = $derived.by(() => {
    if (betaGate !== null) return betaGate;
    if (!user.data.prismAccess) return 'prism_beta_required';
    if (!user.data.entitled) return 'subscription_required';
    if (!aiOptIn) return 'ai_opt_in_required';
    if (user.data.prismCredit.balance <= 0) return 'prism_credit_insufficient';
    return null;
  });
  const accessUnavailableMessage = $derived(accessReason === null ? undefined : prismAccessUnavailableMessage(accessReason));

  let seenGateReason: PrismAccessReason | null = null;

  $effect(() => {
    const reason = accessReason;
    if (reason === seenGateReason) return;
    seenGateReason = reason;
    if (reason !== null) mixpanel.track('view_prism_gate', { reason });
  });

  $effect(() => {
    app.state.prismBadge = user.data.prismSessions.some((session) => hasUnread(session));
  });

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

  const [markPrismSessionSeen] = createMutation(
    graphql(`
      mutation DashboardLayout_PrismPanel_MarkSeen_Mutation($input: MarkPrismSessionSeenInput!) {
        markPrismSessionSeen(input: $input) {
          id
          unseenReviewCount
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
            toolPolicy
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

  const [resolvePrismTool] = createMutation(
    graphql(`
      mutation DashboardLayout_PrismPanel_ResolveTool_Mutation($input: ResolvePrismToolInput!) {
        resolvePrismTool(input: $input) {
          id
        }
      }
    `),
  );

  const [updatePrismSessionToolPolicy] = createMutation(
    graphql(`
      mutation DashboardLayout_PrismPanel_UpdateToolPolicy_Mutation($input: UpdatePrismSessionToolPolicyInput!) {
        updatePrismSessionToolPolicy(input: $input) {
          id
          toolPolicy
        }
      }
    `),
  );

  const selected = createPrismSessionState(user.data.id);
  const sessions = $derived(user.data.prismSessions);
  const currentSession = $derived(sessions.find((session) => session.id === selected.current) ?? null);
  const currentSiteId = $derived((user.data.sites.find((s) => s.id === app.preference.current.currentSiteId) ?? user.data.sites[0])?.id);

  const pendingPolicy = $derived<ToolPolicy>(app.preference.current.prismToolPolicy ?? 'STANDARD');

  const chat = createPrismChat({
    load: fetchTranscript,
    send: async (sessionId, message) => {
      const resp = await sendPrismMessage({
        input: { sessionId: sessionId ?? undefined, message, siteId: currentSiteId, toolPolicy: sessionId ? undefined : pendingPolicy },
      });
      return { sessionId: resp.sendPrismMessage.session.id, runSeq: resp.sendPrismMessage.runSeq };
    },
    cancel: async (sessionId) => {
      await cancelPrismRun({ input: { sessionId } });
    },
  });

  const openDocuments = getOpenDocuments();

  const policy = $derived<ToolPolicy>(currentSession?.toolPolicy ?? pendingPolicy);
  const blocked = $derived(pendingRootRequests(chat.transcript).some((request) => effectiveResolver(request.tool, policy) === 'user'));

  const onPolicyChange = async (next: ToolPolicy) => {
    const sessionId = chat.sessionId;
    if (sessionId === null) {
      app.preference.current.prismToolPolicy = next;
      mixpanel.track('change_prism_tool_policy', { policy: next, scope: 'default' });
      return;
    }

    try {
      await updatePrismSessionToolPolicy(
        { input: { sessionId, policy: next } },
        { metadata: { cache: { optimisticResponse: { updatePrismSessionToolPolicy: { id: sessionId, toolPolicy: next } } } } },
      );
    } catch {
      Toast.error('설정을 바꾸지 못했어요. 잠시 후 다시 시도해 주세요');
      return;
    }

    mixpanel.track('change_prism_tool_policy', { policy: next, scope: 'session' });
  };

  const commands = $derived(
    user.data.prismCommands?.map((command) => ({
      name: command.name,
      description: command.description,
      argumentHint: command.argumentHint ?? null,
    })) ?? null,
  );

  const activeWorkflow = $derived(runningWorkflows(chat.transcript).at(-1) ?? null);
  const workflowCopy = $derived(activeWorkflow === null ? null : (workflowApps[activeWorkflow.app]?.composer ?? null));
  const awaitingAnswer = $derived(
    activeWorkflow !== null &&
      chat.transcript.messages.some(
        (m) => m.role === 'tool-request' && m.workflowId === activeWorkflow.workflowId && m.status === 'pending',
      ),
  );
  const composerStatus = $derived.by(() => {
    if (activeWorkflow === null) return null;
    if (workflowCopy === null) return { text: awaitingAnswer ? '위 질문에 답하면 작업이 이어져요' : '작업이 진행 중이에요', stop: null };
    return { text: awaitingAnswer ? workflowCopy.waiting : workflowCopy.running, stop: workflowCopy.stop };
  });

  const resolveToolForSession = async (
    sessionId: string | null,
    transcriptAgentId: string | null,
    agentId: string,
    toolCallId: string,
    input: unknown,
  ) => {
    if (sessionId === null) {
      throw new Error('prism session is not ready');
    }

    const root = agentId.length === 0 || agentId === transcriptAgentId;
    await resolvePrismTool({ input: { sessionId, agentId: root ? undefined : agentId, toolCallId, input } });
  };

  const resolveTool = async (agentId: string, toolCallId: string, input: unknown) => {
    await resolveToolForSession(chat.sessionId, chat.transcript.agentId, agentId, toolCallId, input);
  };

  const autoResolver = new AutoResolver({
    resolve: async (toolCallId) => {
      const sessionId = chat.sessionId;
      if (sessionId === null) {
        return;
      }

      const transcriptAgentId = chat.transcript.agentId;
      const request = pendingRootRequests(chat.transcript).find((entry) => entry.toolCallId === toolCallId);
      const resolver = request === undefined ? undefined : clientResolvers[request.tool];
      if (request === undefined || resolver === undefined) {
        return;
      }

      const input = await resolver({ openDocuments });
      await resolveToolForSession(sessionId, transcriptAgentId, request.agentId, toolCallId, input);
    },
    settled: (err) => {
      const error = unwrapError(err);
      return error instanceof TypieError && error.code === 'prism_tool_settled';
    },
  });

  $effect(() => {
    void chat.generation;
    return () => {
      autoResolver.reset();
      resetReconnect();
    };
  });

  $effect(() => {
    if (accessReason !== null) {
      untrack(() => autoResolver.reset());
      return;
    }

    const sessionId = chat.sessionId;
    const requests = pendingRootRequests(chat.transcript).filter((request) => clientResolvers[request.tool] !== undefined);
    if (sessionId === null || chat.loading) {
      return;
    }

    untrack(() => {
      autoResolver.retain(requests.map((request) => request.toolCallId));
      for (const request of requests) {
        autoResolver.request(request.toolCallId);
      }
    });
  });

  let composer = $state<PrismComposer>();
  let draft = $state('');
  const emptySession = $derived(chat.transcript.messages.length === 0 && !chat.transcript.live && chat.pending === null);
  let indicatorDestination = $state<HTMLElement>();
  let indicatorGeneration = chat.generation;
  let indicatorPhase = $state<'answered' | 'failed' | 'hidden' | 'submitting' | 'welcome'>(
    selected.current === null ? 'welcome' : 'hidden',
  );
  let indicatorSpinnerPlaybackStartedAt = $state<number | null>();
  let indicatorWaitSeen = $state(false);
  const chipsVisible = $derived(emptySession && draft.length === 0);
  const chipClass = css({
    display: 'flex',
    alignItems: 'center',
    gap: '5px',
    height: '28px',
    paddingX: '12px',
    borderRadius: 'full',
    fontSize: '12px',
    fontWeight: 'medium',
    color: 'text.subtle',
    backgroundColor: 'surface.muted',
    transition: '[background-color 150ms ease]',
    _hover: { backgroundColor: 'interactive.hover' },
  });
  let listOpen = $state(false);
  const listToggleLabel = $derived(listOpen ? '대화 목록 닫기' : '대화 목록 열기');
  const currentTitle = $derived(currentSession ? sessionLabel(currentSession) : '새 대화');
  let currentTitleButton = $state<HTMLButtonElement>();
  let currentTitleClientWidth = $state(0);
  let currentTitleTruncated = $state(false);

  $effect(() => {
    const element = currentTitleButton;
    void currentTitle;
    void currentTitleClientWidth;
    if (!element) return;

    let cancelled = false;
    void tick().then(() => {
      if (!cancelled) currentTitleTruncated = element.scrollWidth > element.clientWidth;
    });

    return () => {
      cancelled = true;
    };
  });

  export const startNewChat = async (via: 'command_palette' | 'header', nextDraft?: string) => {
    selected.current = null;
    listOpen = false;
    if (nextDraft !== undefined) draft = nextDraft;

    mixpanel.track('open_prism_new_chat', { via, with_draft: nextDraft !== undefined });

    if (!app.preference.current.prismPanelOpen) {
      app.preference.current.prismPanelOpen = true;
      mixpanel.track('open_prism_panel', { via: 'new_chat' });
    }

    await tick();
    composer?.focus();
  };

  let seenTitle: string | null = null;

  $effect.pre(() => {
    const generation = chat.generation;
    if (generation === indicatorGeneration) return;
    indicatorGeneration = generation;
    untrack(() => {
      indicatorDestination = undefined;
      indicatorPhase = selected.current === null ? 'welcome' : 'hidden';
      indicatorSpinnerPlaybackStartedAt = undefined;
      indicatorWaitSeen = false;
    });
  });

  $effect(() => {
    const anchor = indicatorDestination;
    if (indicatorPhase !== 'submitting') return;
    if (anchor) {
      indicatorWaitSeen = true;
    } else if (indicatorWaitSeen) {
      indicatorPhase = 'answered';
    }
  });

  $effect(() => {
    const title = chat.transcript.title;
    if (title === seenTitle) {
      return;
    }

    seenTitle = title;
    if (title !== null && untrack(() => currentSession?.title) !== title) {
      cache.invalidate({ __typename: 'User', id: user.data.id, $field: 'prismSessions' });
    }
  });

  const archiveSession = async (id: string, via: 'header_menu' | 'session_list') => {
    try {
      await archivePrismSession(
        { input: { sessionId: id } },
        { metadata: { cache: { optimisticResponse: { archivePrismSession: { id, archivedAt: new Date().toISOString() } } } } },
      );
    } catch {
      Toast.error('보관하지 못했어요. 잠시 후 다시 시도해 주세요');
      return;
    }

    mixpanel.track('archive_prism_session', { via });

    if (selected.current === id) {
      selected.current = null;
    }
  };

  const unarchiveSession = async (id: string, via: 'header_menu' | 'session_list') => {
    try {
      await unarchivePrismSession(
        { input: { sessionId: id } },
        { metadata: { cache: { optimisticResponse: { unarchivePrismSession: { id, archivedAt: null } } } } },
      );
    } catch {
      Toast.error('복원하지 못했어요. 잠시 후 다시 시도해 주세요');
      return;
    }

    mixpanel.track('unarchive_prism_session', { via });
  };

  const renameSession = async (id: string, title: string, via: 'header_menu' | 'session_list') => {
    try {
      await renamePrismSession(
        { input: { sessionId: id, title } },
        { metadata: { cache: { optimisticResponse: { renamePrismSession: { id, title } } } } },
      );
    } catch {
      Toast.error('이름을 바꾸지 못했어요. 잠시 후 다시 시도해 주세요');
      return;
    }

    mixpanel.track('rename_prism_session', { via });
  };

  const requestDelete = (session: { id: string; title?: string | null }, via: 'header_menu' | 'session_list') => {
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

        mixpanel.track('delete_prism_session', { via });

        if (selected.current === session.id) {
          selected.current = null;
        }

        cache.invalidate({ __typename: 'User', id: user.data.id, $field: 'prismSessions' });
      },
    });
  };

  let titleEditing = $state(false);
  let titleDraft = $state('');
  let titleInput = $state<HTMLInputElement>();
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
    await renameSession(session.id, title, 'header_menu');
  };

  $effect(() => {
    void selected.current;
    titleEditing = false;
  });

  // 리뷰 모달의 "대화 보기" — 패널은 닫혀 있어도 늘 마운트되어 있으므로 여기서 요청을 가져간다
  $effect(() => {
    const sessionId = takeSessionJump();
    if (sessionId === null) return;
    untrack(() => {
      selected.current = sessionId;
      if (!app.preference.current.prismPanelOpen) {
        app.preference.current.prismPanelOpen = true;
        mixpanel.track('open_prism_panel', { via: 'review_jump' });
      }
    });
  });

  const calloutStyle = flex.raw({
    alignItems: 'center',
    gap: '8px',
    paddingX: '12px',
    paddingY: '10px',
    borderWidth: '1px',
    borderRadius: '10px',
  });
  const calloutNeutralStyle = css.raw({ borderColor: 'border.subtle', backgroundColor: 'surface.subtle' });
  const calloutDangerStyle = css.raw({ borderColor: 'border.danger', backgroundColor: 'accent.danger.subtle' });
  const calloutTextClass = css({ flexGrow: '1', minWidth: '0', fontSize: '12px', lineHeight: '[1.5]', color: 'text.subtle' });

  const panelOpen = $derived(app.preference.current.prismPanelOpen);
  const panelVisible = $derived(app.state.prismAccess || panelOpen);
  const panelInteractive = $derived(panelOpen && !app.preference.current.zenModeEnabled);
  const welcomeAdmission = $derived(panelInteractive && !listOpen && page.state.shallowRoute == null);
  let prevPanelOpen: boolean | null = null;

  $effect(() => {
    const open = panelOpen;
    const transition = prevPanelOpen === false && open;
    prevPanelOpen = open;

    if (transition && panelInteractive) composer?.focus();
  });

  const markSeen = (sessionId: string) => {
    void markPrismSessionSeen({ input: { sessionId } }).catch(() => null);
  };

  $effect(() => {
    if (!app.preference.current.prismPanelOpen) {
      return;
    }

    const id = selected.current;
    untrack(() => {
      void chat.load(id);
      if (id !== null) markSeen(id);
    });
  });

  $effect(() => {
    const session = currentSession;
    if (session === null || session.unseenReviewCount === 0 || !app.preference.current.prismPanelOpen) {
      return;
    }

    untrack(() => markSeen(session.id));
  });

  const RECONNECT_MS = [1000, 3000, 10_000, 30_000];
  const SOCKET_DOWN_GRACE_MS = 1500;

  let subscribeCursor = $state(0);
  let subscribeWorkflows = $state<{ workflowId: string; cursor: number }[]>([]);
  let reconnecting = $state(false);
  let reconnectFailed = $state(false);
  let reconnectAttempts = 0;
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

  const workflowCursors = () =>
    chat.transcript.messages
      .filter((message): message is WorkflowMessage => message.role === 'workflow' && message.status === 'running')
      .map((message) => ({ workflowId: message.workflowId, cursor: message.cursor }));

  $effect(() => {
    void chat.seedCursor;
    untrack(() => {
      subscribeCursor = chat.transcript.cursor;
      subscribeWorkflows = workflowCursors();
    });
  });

  const clearReconnectTimer = () => {
    if (reconnectTimer !== null) {
      clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }
  };

  const resetReconnect = () => {
    clearReconnectTimer();
    reconnectAttempts = 0;
    reconnecting = false;
    reconnectFailed = false;
  };

  const scheduleReconnect = () => {
    clearReconnectTimer();
    reconnectAttempts += 1;

    const delay = backoffDelay(RECONNECT_MS, reconnectAttempts);
    if (delay === null) {
      reconnecting = false;
      reconnectFailed = true;

      try {
        Sentry.captureMessage('prism session reconnect exhausted', {
          level: 'error',
          extra: { attempts: reconnectAttempts },
        });
      } catch {
        // 보고 실패가 재연결 상태를 바꾸지 않는다
      }

      return;
    }

    reconnecting = true;
    reconnectTimer = setTimeout(() => {
      reconnectTimer = null;
      subscribeCursor = chat.transcript.cursor;
      subscribeWorkflows = workflowCursors();
      reconnecting = false;
    }, delay);
  };

  const subscriptionSkipped = $derived(
    chat.sessionId === null || !app.preference.current.prismPanelOpen || chat.loading || reconnecting || reconnectFailed,
  );

  const socketDown = $derived(!subscriptionSkipped && wsStatus.current !== 'connected');
  let socketDownSettled = $state(false);

  $effect(() => {
    if (!socketDown) {
      socketDownSettled = false;
      return;
    }

    const id = setTimeout(() => (socketDownSettled = true), SOCKET_DOWN_GRACE_MS);
    return () => clearTimeout(id);
  });

  const disconnected = $derived(reconnecting || socketDownSettled);

  const statusKind = $derived(chat.error ? 'error' : disconnected ? 'reconnecting' : reconnectFailed ? 'failed' : null);

  let statusEl = $state<HTMLElement>();
  let statusFrom = $state<number>();
  let prevStatusKind: string | null | undefined;

  $effect.pre(() => {
    const next = statusKind;
    if (prevStatusKind !== undefined && prevStatusKind !== null && next !== null && next !== prevStatusKind) {
      statusFrom = statusEl?.offsetHeight;
    }
    prevStatusKind = next;
  });

  createSubscription(
    graphql(`
      subscription DashboardLayout_PrismPanel_Events_Subscription($sessionId: ID!, $cursor: Int, $workflows: [PrismWorkflowCursorInput!]) {
        prismSessionEvents(cursor: $cursor, sessionId: $sessionId, workflows: $workflows)
      }
    `),
    () => ({ sessionId: chat.sessionId ?? '', cursor: subscribeCursor, workflows: subscribeWorkflows }),
    () => ({
      skip: subscriptionSkipped,
      onData: (data) => {
        if (reconnectAttempts !== 0 || reconnecting || reconnectFailed) {
          resetReconnect();
        }

        chat.receive(toFrame(data.prismSessionEvents));
      },
      onError: (err) => {
        const error = unwrapError(err);
        if (error instanceof TypieError && error.code === 'prism_beta_required') {
          betaGate = 'prism_beta_required';
          return;
        }

        scheduleReconnect();
      },
    }),
  );

  const onSend = async (text: string) => {
    const creating = chat.sessionId === null;
    indicatorWaitSeen = false;
    indicatorSpinnerPlaybackStartedAt = undefined;
    indicatorPhase = 'submitting';

    try {
      const result = await chat.send(text);
      selected.current = result.sessionId;

      const gate = commandGate(text, commands);
      mixpanel.track('send_prism_message', {
        new_session: creating,
        command: gate === 'plain' ? null : gate === 'ok' ? commandNameOf(text) : 'unknown',
        policy,
      });

      if (creating) {
        cache.invalidate({ __typename: 'User', id: user.data.id, $field: 'prismSessions' });
      }
    } catch (err) {
      const error = unwrapError(err);
      const code = error instanceof TypieError ? error.code : null;

      try {
        Sentry.captureMessage('prism message send failed', {
          level: code === null ? 'error' : 'info',
          extra: { code: code ?? 'unknown' },
        });
      } catch {
        // 보고 실패가 전송 결과를 바꾸지 않는다
      }

      if (code === 'prism_beta_required') {
        betaGate = 'prism_beta_required';
      } else if (code === 'prism_credit_insufficient') {
        Toast.error('크레딧이 부족해요');
      } else if (code === 'prism_run_active') {
        Toast.error('답변이 끝난 뒤에 보낼 수 있어요');
      } else if (code === 'prism_unknown_command') {
        Toast.error('등록되지 않은 명령이에요');
      } else {
        Toast.error('메시지를 보내지 못했어요. 잠시 후 다시 시도해 주세요');
      }

      indicatorPhase = 'failed';

      throw err;
    }
  };

  const onChipInsert = (chip: StartChip) => {
    draft = chip.insert;
    mixpanel.track('insert_prism_chip', { label: chip.label });
    composer?.focus();
  };

  const cancelRun = async () => {
    try {
      await chat.stop();
      mixpanel.track('stop_prism_run', { workflow: activeWorkflow?.app ?? null });
      return true;
    } catch {
      Toast.error('중단하지 못했어요. 잠시 후 다시 시도해 주세요');
      return false;
    }
  };

  const onStop = async () => {
    if (activeWorkflow !== null) {
      Dialog.confirm({
        title: workflowCopy?.confirmTitle ?? '진행 중인 작업을 중단할까요?',
        message: workflowCopy?.confirmMessage ?? '진행 중인 작업이 멈춰요. 다시 하려면 새로 시작해야 해요.',
        action: 'danger',
        actionLabel: '중단',
        actionHandler: async () => {
          if (!(await cancelRun())) return false;
        },
      });

      return;
    }

    await cancelRun();
  };
</script>

{#snippet currentTitleTooltip()}
  <span class={css({ display: 'block', maxWidth: '280px', overflowWrap: 'break-word', textWrap: 'balance', wordBreak: 'keep-all' })}>
    {currentTitle}
  </span>
{/snippet}

{#if panelVisible}
  <PrismPanelHeader>
    {#snippet children(buttonClass)}
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
          bind:this={currentTitleButton}
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
          aria-label={`${listToggleLabel}: ${currentTitle}`}
          onclick={() => (listOpen = !listOpen)}
          type="button"
          bind:clientWidth={currentTitleClientWidth}
          use:tooltip={{ message: currentTitleTruncated ? currentTitleTooltip : listToggleLabel, arrow: !currentTitleTruncated }}
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
            <div
              class={cx(buttonClass, open ? css({ color: 'text.default', backgroundColor: 'surface.muted' }) : undefined)}
              aria-label="대화 메뉴"
              use:tooltip={{ message: '대화 메뉴' }}
            >
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
                  void archiveSession(session.id, 'header_menu');
                }}
              >
                보관
              </MenuItem>
            {:else}
              <MenuItem
                icon={ArchiveRestoreIcon}
                onclick={() => {
                  close();
                  void unarchiveSession(session.id, 'header_menu');
                }}
              >
                복원
              </MenuItem>
            {/if}
            <MenuItem
              icon={TrashIcon}
              onclick={() => {
                close();
                requestDelete(session, 'header_menu');
              }}
              variant="danger"
            >
              삭제
            </MenuItem>
          {/snippet}
        </Menu>
      {/if}

      <button
        class={buttonClass}
        aria-label="새 대화"
        onclick={() => void startNewChat('header')}
        type="button"
        use:tooltip={{ message: '새 대화' }}
      >
        <Icon icon={PlusIcon} size={16} />
      </button>

      <button
        class={cx(buttonClass, listOpen ? css({ color: 'text.default', backgroundColor: 'surface.muted' }) : undefined)}
        aria-label="대화 목록"
        aria-pressed={listOpen}
        onclick={() => (listOpen = !listOpen)}
        type="button"
        use:tooltip={{ message: listToggleLabel }}
      >
        <span class={css({ position: 'relative', display: 'flex', flexShrink: '0' })}>
          <Icon icon={HistoryIcon} size={16} />
          {#if app.state.prismBadge}
            <PrismBadgeDot />
          {/if}
        </span>
      </button>
    {/snippet}
  </PrismPanelHeader>

  <div class={flex({ position: 'relative', flexDirection: 'column', flexGrow: '1', minHeight: '0' })}>
    {#if listOpen}
      <PrismSessionList
        currentId={selected.current}
        onArchive={(id) => archiveSession(id, 'session_list')}
        onClose={() => (listOpen = false)}
        onDelete={(session) => requestDelete(session, 'session_list')}
        onRename={(id, title) => renameSession(id, title, 'session_list')}
        onSelect={(id) => {
          selected.current = id;
          listOpen = false;
        }}
        onUnarchive={(id) => unarchiveSession(id, 'session_list')}
        {sessions}
      />
    {/if}

    <div class={flex({ position: 'relative', flexDirection: 'column', flexGrow: '1', minHeight: '0' })}>
      {#if app.preference.current.prismPanelOpen && indicatorPhase !== 'hidden'}
        {#key chat.generation}
          <PrismPanelIndicator
            destination={indicatorDestination}
            phase={indicatorPhase}
            prismEnabled={app.preference.current.prismWelcomeObjectEnabled}
            reducedMotion={prefersReducedMotion.current}
            rowSpinnerPlaybackStartedAt={indicatorSpinnerPlaybackStartedAt}
            themeVariant={theme.currentThemeVariant}
            {welcomeAdmission}
          />
        {/key}
      {/if}

      {#key chat.generation}
        <PrismTranscript
          failedIds={autoResolver.failedIds}
          loading={chat.loading}
          onResolve={resolveTool}
          onRetry={(toolCallId) => autoResolver.retry(toolCallId)}
          onSpinnerPlaybackChange={(startedAt) => (indicatorSpinnerPlaybackStartedAt = startedAt)}
          pending={chat.pending}
          {policy}
          reconnecting={disconnected}
          sessionId={chat.sessionId}
          transcript={chat.transcript}
          unavailableMessage={accessUnavailableMessage}
          bind:waitSpinnerAnchor={indicatorDestination}
        />
      {/key}

      {#if statusKind !== null}
        <div class={css({ paddingX: '12px', paddingBottom: '8px' })} transition:expand>
          <div bind:this={statusEl} class={css({ position: 'relative', zIndex: '2' })}>
            {#if statusKind === 'reconnecting'}
              <div class={css(calloutStyle, calloutNeutralStyle)} in:swap={{ box: statusEl, from: statusFrom }}>
                <PrismSpinner label="다시 연결하는 중" />
                <span class={calloutTextClass}>다시 연결하는 중이에요</span>
              </div>
            {:else if statusKind === 'error'}
              <div class={css(calloutStyle, calloutDangerStyle)} in:swap={{ box: statusEl, from: statusFrom }}>
                <Icon style={css.raw({ flexShrink: '0', color: 'text.danger' })} icon={CircleAlertIcon} size={14} />
                <span class={calloutTextClass}>{chat.error}</span>
                <Button style={css.raw({ flexShrink: '0' })} onclick={() => void chat.load(selected.current)} size="sm" variant="secondary">
                  다시 불러오기
                </Button>
              </div>
            {:else}
              <div class={css(calloutStyle, calloutDangerStyle)} in:swap={{ box: statusEl, from: statusFrom }}>
                <Icon style={css.raw({ flexShrink: '0', color: 'text.danger' })} icon={CircleAlertIcon} size={14} />
                <span class={calloutTextClass}>실시간 연결이 끊겼어요</span>
                <Button style={css.raw({ flexShrink: '0' })} onclick={resetReconnect} size="sm" variant="secondary">다시 연결</Button>
              </div>
            {/if}
          </div>
        </div>
      {/if}

      {#if !chat.loading && emptySession}
        <div
          class={css(
            { paddingX: '12px', paddingBottom: '24px', transition: '[opacity 150ms ease]' },
            chipsVisible ? {} : { opacity: '0', pointerEvents: 'none' },
          )}
          aria-hidden={!chipsVisible}
          in:rise
        >
          <p class={css({ marginBottom: '6px', fontSize: '13px', fontWeight: 'semibold', color: 'text.faint' })}>제안</p>
          <div class={flex({ position: 'relative', zIndex: '2', flexWrap: 'wrap', gap: '6px' })}>
            {#each startChipsFor(openDocuments.snapshot().documents.length > 0) as chip (chip.insert)}
              <button class={chipClass} onclick={() => onChipInsert(chip)} tabindex={chipsVisible ? 0 : -1} type="button">
                <Icon icon={chip.icon} size={14} />
                {chip.label}
              </button>
            {/each}
          </div>
        </div>
      {/if}
    </div>

    <PrismPushCard visible={app.state.prismBadge} />

    {#if accessReason !== null}
      <PrismGateCard reason={accessReason} />
    {/if}

    {#if !chat.loading}
      <PrismComposer
        bind:this={composer}
        {blocked}
        {commands}
        {onSend}
        {onStop}
        policy={{ current: policy, onChange: onPolicyChange }}
        running={chat.transcript.run === 'running'}
        sendDisabled={accessReason !== null}
        status={composerStatus}
        bind:text={draft}
      />
    {/if}
  </div>
{/if}
