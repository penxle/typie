<script lang="ts">
  import { createMutation, createQuery, createSubscription } from '@mearie/svelte';
  import { Toast } from '@typie/ui/notification';
  import { onDestroy } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';
  import MessageSquarePlusIcon from '~icons/lucide/message-square-plus';
  import { reconcileComments } from '$lib/editor-ffi/comments';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { FocusReturnSession } from '$lib/focus-return-session';
  import { cache } from '$lib/graphql';
  import { graphql } from '$mearie';
  import { setupCommentContext } from './context.svelte';
  import type { PageRect, StableSelection } from '@typie/editor-ffi/browser';
  import type { Snippet } from 'svelte';
  import type { CommentComposerV2_user$key } from '$mearie';
  import type { CommentAnchor, CommentThread } from './context.svelte';

  type Props = {
    documentId: string;
    entityId: string;
    myId: string;
    isOwner: boolean;
    me$key: CommentComposerV2_user$key;
    children: Snippet;
  };
  let { documentId, entityId, myId, isOwner, me$key, children }: Props = $props();

  const ctx = getEditorContext();
  const clientId = crypto.randomUUID();
  const editor = $derived(ctx.editor);

  let activeThreadId = $state<string | null>(null);
  let activeAnchor = $state<CommentAnchor | null>(null);
  let composing = $state(false);
  let showResolved = $state(false);
  let composeFrozen: StableSelection | null = null;
  let pendingThread = $state<CommentThread | null>(null);
  let focusReturnSession: FocusReturnSession | null = null;
  let focusReturnRegion: HTMLElement | null = null;
  const justCreated = new SvelteSet<string>();

  createSubscription(
    graphql(`
      subscription DocumentCommentsV2_Stream($documentId: ID!, $clientId: String!) {
        documentCommentStream(documentId: $documentId, clientId: $clientId) {
          id
          selection
          ...DocumentPanelV2CommentItem_thread
          ...CommentPopoverV2_thread
        }
      }
    `),
    () => ({ documentId, clientId }),
    () => ({
      onData: () => {
        cache.invalidate({ __typename: 'Document', id: documentId, $field: 'commentThreads' });
      },
    }),
  );

  const openQuery = createQuery(
    graphql(`
      query DocumentCommentsV2_Open_Query($entityId: ID!) {
        entity(entityId: $entityId) {
          id

          node {
            __typename

            ... on Document {
              id

              commentThreads(resolved: false) {
                id
                selection
                ...DocumentPanelV2CommentItem_thread
                ...CommentPopoverV2_thread
              }
            }
          }
        }
      }
    `),
    () => ({ entityId }),
  );

  const resolvedQuery = createQuery(
    graphql(`
      query DocumentCommentsV2_Resolved_Query($entityId: ID!) {
        entity(entityId: $entityId) {
          id

          node {
            __typename

            ... on Document {
              id

              commentThreads(resolved: true) {
                id
                selection
                ...DocumentPanelV2CommentItem_thread
                ...CommentPopoverV2_thread
              }
            }
          }
        }
      }
    `),
    () => ({ entityId }),
    () => ({ skip: !showResolved }),
  );

  const docNode = $derived(openQuery.data?.entity.node);
  const threads = $derived((docNode?.__typename === 'Document' ? docNode.commentThreads : []) as unknown as CommentThread[]);
  const resolvedNode = $derived(resolvedQuery.data?.entity.node);
  const resolvedThreads = $derived(
    (resolvedNode?.__typename === 'Document' ? resolvedNode.commentThreads : []) as unknown as CommentThread[],
  );
  const activeThread = $derived(
    activeThreadId
      ? (threads.find((t) => t.id === activeThreadId) ?? (pendingThread?.id === activeThreadId ? pendingThread : undefined))
      : undefined,
  );

  $effect(() => {
    if (!editor) return;
    const desired = threads.map((t) => t.id);
    const { toAdd, toRemove } = reconcileComments(editor.registeredCommentIds(), desired);
    for (const id of toRemove) editor.removeComment(id);
    for (const id of toAdd) {
      const t = threads.find((x) => x.id === id);
      if (t) editor.addFrozenComment(id, t.selection as StableSelection);
    }
  });

  $effect(() => {
    if (!editor) return;
    void editor.trackedRanges;
    if (activeThreadId && editor.hasComment(activeThreadId)) {
      editor.setActiveComment(activeThreadId);
    } else if (!activeThreadId) {
      editor.setActiveComment(null);
    }
  });

  $effect(() => {
    for (const id of justCreated) {
      if (threads.some((t) => t.id === id)) justCreated.delete(id);
    }
    const pt = pendingThread;
    if (pt && threads.some((t) => t.id === pt.id)) pendingThread = null;
  });

  $effect.pre(() => {
    if (composing) return;
    if (activeThreadId && !justCreated.has(activeThreadId) && threads.every((t) => t.id !== activeThreadId)) {
      closeAutomatically();
    }
  });

  $effect(() => {
    if (!editor) return;
    const ed = editor;
    ed.commentClickHandler = (id) => {
      openThread(id);
    };
    return () => {
      ed.commentClickHandler = null;
    };
  });

  $effect(() => {
    if (!editor) return;
    const ed = editor;
    ed.requestCommentCompose = () => startComposing();
    const off = ed.registerContextMenuContributor(() => {
      if (ed.isSelectionCollapsed) return [];
      return [{ label: '코멘트 달기', icon: MessageSquarePlusIcon, onclick: () => startComposing() }];
    });
    return () => {
      ed.requestCommentCompose = null;
      off();
    };
  });

  const anchorFromPageRects = (rects: PageRect[]): CommentAnchor | null => (rects.length > 0 ? { rects } : null);

  const anchorForThread = (id: string): CommentAnchor | null => {
    if (!editor) return null;
    const rects = editor.trackedItemRects(id);
    return rects ? anchorFromPageRects(rects) : null;
  };

  function isLocatable(id: string): boolean {
    return editor?.isCommentLocatable(id) ?? false;
  }

  function startComposing() {
    if (!editor || editor.isSelectionCollapsed || !editor.selection) return;
    const frozen = editor.freezeSelection(editor.selection);
    if (!frozen) {
      Toast.error('선택 영역에 코멘트를 달 수 없어요');
      return;
    }
    composeFrozen = frozen;
    editor.setCommentComposeRange(frozen);
    composing = true;
    activeThreadId = null;

    // TODO: compose range를 active anchor로 쓰고, compose range로 scrollIntoView 하기
    if (editor.cursor) {
      activeAnchor = { rects: [{ page_idx: editor.cursor.page_idx, rect: editor.cursor.caret }] };
    } else {
      const rect = editor.selectionHeadRect();
      activeAnchor = rect ? anchorFromPageRects([rect]) : null;
      ctx.scroll?.scrollIntoView({ target: { type: 'current_selection_head' } });
    }
  }

  function clearCompose() {
    composeFrozen = null;
    editor?.setCommentComposeRange(null);
  }

  function openThread(id: string, anchor?: CommentAnchor) {
    composing = false;
    clearCompose();
    if (pendingThread && pendingThread.id !== id) pendingThread = null;
    activeThreadId = id;
    activeAnchor = anchor ?? anchorForThread(id) ?? activeAnchor;
  }

  function openFromPanel(id: string) {
    if (!editor) return;
    if (!isLocatable(id)) {
      Toast.error('원문에서 위치를 찾을 수 없는 코멘트예요');
      return;
    }
    composing = false;
    const rects = editor.trackedItemRects(id);
    const anchor = rects ? anchorFromPageRects(rects) : null;
    if (!anchor) {
      Toast.error('원문에서 위치를 찾을 수 없는 코멘트예요');
      return;
    }
    activeThreadId = id;
    activeAnchor = anchor;
    ctx.scroll?.scrollIntoView({ target: { type: 'tracked_item', id } });
  }

  function takeFocusReturnSession(): FocusReturnSession | null {
    const session = focusReturnSession;
    focusReturnSession = null;
    return session;
  }

  function clearCommentUi() {
    composing = false;
    clearCompose();
    pendingThread = null;
    justCreated.clear();
    activeThreadId = null;
    activeAnchor = null;
  }

  function close() {
    const session = takeFocusReturnSession();
    clearCommentUi();
    session?.restore();
  }

  function closeFromOutside() {
    const session = takeFocusReturnSession();
    clearCommentUi();
    session?.discard();
  }

  function closeAutomatically() {
    const session = takeFocusReturnSession();
    if (focusReturnRegion) session?.restoreIfFocusWithin(focusReturnRegion);
    else session?.discard();
    clearCommentUi();
  }

  function captureFocusReturn(target: EventTarget | null) {
    focusReturnSession ??= FocusReturnSession.capture(target);
  }

  function setFocusReturnRegion(region: HTMLElement | null) {
    focusReturnRegion = region;
  }

  const [createThreadMutation] = createMutation(
    graphql(`
      mutation DocumentCommentsV2_CreateThread($input: CreateDocumentCommentThreadInput!) {
        createDocumentCommentThread(input: $input) {
          id
          selection
          ...DocumentPanelV2CommentItem_thread
          ...CommentPopoverV2_thread
        }
      }
    `),
  );
  const [createCommentMutation] = createMutation(
    graphql(`
      mutation DocumentCommentsV2_CreateComment($input: CreateDocumentCommentInput!) {
        createDocumentComment(input: $input) {
          id
          selection
          ...DocumentPanelV2CommentItem_thread
          ...CommentPopoverV2_thread
        }
      }
    `),
  );
  const [updateCommentMutation] = createMutation(
    graphql(`
      mutation DocumentCommentsV2_UpdateComment($input: UpdateDocumentCommentInput!) {
        updateDocumentComment(input: $input) {
          id
          selection
          ...DocumentPanelV2CommentItem_thread
          ...CommentPopoverV2_thread
        }
      }
    `),
  );
  const [deleteCommentMutation] = createMutation(
    graphql(`
      mutation DocumentCommentsV2_DeleteComment($input: DeleteDocumentCommentInput!) {
        deleteDocumentComment(input: $input) {
          id
          selection
          ...DocumentPanelV2CommentItem_thread
          ...CommentPopoverV2_thread
        }
      }
    `),
  );
  const [deleteThreadMutation] = createMutation(
    graphql(`
      mutation DocumentCommentsV2_DeleteThread($input: DeleteDocumentCommentThreadInput!) {
        deleteDocumentCommentThread(input: $input) {
          id
          selection
          ...DocumentPanelV2CommentItem_thread
          ...CommentPopoverV2_thread
        }
      }
    `),
  );
  const [resolveThreadMutation] = createMutation(
    graphql(`
      mutation DocumentCommentsV2_ResolveThread($input: ResolveDocumentCommentThreadInput!) {
        resolveDocumentCommentThread(input: $input) {
          id
          selection
          ...DocumentPanelV2CommentItem_thread
          ...CommentPopoverV2_thread
        }
      }
    `),
  );
  const [unresolveThreadMutation] = createMutation(
    graphql(`
      mutation DocumentCommentsV2_UnresolveThread($input: UnresolveDocumentCommentThreadInput!) {
        unresolveDocumentCommentThread(input: $input) {
          id
          selection
          ...DocumentPanelV2CommentItem_thread
          ...CommentPopoverV2_thread
        }
      }
    `),
  );

  const invalidateMembership = () => {
    cache.invalidate({ __typename: 'Document', id: documentId, $field: 'commentThreads' });
  };

  async function createThread(content: string) {
    if (!editor || !composeFrozen) return;
    const frozen = composeFrozen;
    try {
      const res = await createThreadMutation({ input: { documentId, selection: frozen, content, clientId } });
      const created = res.createDocumentCommentThread as unknown as CommentThread;
      const id = created.id;
      pendingThread = created;
      justCreated.add(id);
      invalidateMembership();
      openThread(id);
    } catch (err) {
      Toast.error('코멘트 작성에 실패했어요');
      throw err;
    }
  }
  async function reply(threadId: string, content: string) {
    try {
      await createCommentMutation({ input: { threadId, content, clientId } });
    } catch (err) {
      Toast.error('답글 작성에 실패했어요');
      throw err;
    }
  }
  async function editComment(commentId: string, content: string) {
    try {
      await updateCommentMutation({ input: { commentId, content, clientId } });
    } catch (err) {
      Toast.error('수정에 실패했어요');
      throw err;
    }
  }
  async function deleteComment(commentId: string) {
    try {
      await deleteCommentMutation({ input: { commentId, clientId } });
    } catch {
      Toast.error('삭제에 실패했어요');
    }
  }
  async function deleteThread(threadId: string) {
    try {
      await deleteThreadMutation({ input: { threadId, clientId } });
      invalidateMembership();
      if (activeThreadId === threadId) close();
    } catch {
      Toast.error('삭제에 실패했어요');
    }
  }
  async function resolveThread(threadId: string) {
    try {
      await resolveThreadMutation({ input: { threadId, clientId } });
      invalidateMembership();
      if (activeThreadId === threadId) close();
    } catch {
      Toast.error('해결 처리에 실패했어요');
    }
  }
  async function unresolveThread(threadId: string) {
    try {
      await unresolveThreadMutation({ input: { threadId, clientId } });
      invalidateMembership();
    } catch {
      Toast.error('해결 취소에 실패했어요');
    }
  }

  setupCommentContext({
    get threads() {
      return threads;
    },
    get resolvedThreads() {
      return resolvedThreads;
    },
    get showResolved() {
      return showResolved;
    },
    get activeThreadId() {
      return activeThreadId;
    },
    get activeThread() {
      return activeThread;
    },
    get activeAnchor() {
      return activeAnchor;
    },
    get composing() {
      return composing;
    },
    get myId() {
      return myId;
    },
    get isOwner() {
      return isOwner;
    },
    get meUser() {
      return me$key;
    },
    setShowResolved: (v) => {
      showResolved = v;
    },
    isLocatable,
    openThread,
    openFromPanel,
    captureFocusReturn,
    setFocusReturnRegion,
    close,
    closeFromOutside,
    createThread,
    reply,
    editComment,
    deleteComment,
    deleteThread,
    resolveThread,
    unresolveThread,
  });

  onDestroy(() => {
    takeFocusReturnSession()?.discard();
    focusReturnRegion = null;
    editor?.setActiveComment(null);
  });
</script>

{@render children()}
