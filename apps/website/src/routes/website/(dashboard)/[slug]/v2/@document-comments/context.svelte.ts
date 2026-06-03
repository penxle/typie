import { getContext, setContext } from 'svelte';
import type { PageRect } from '@typie/editor-ffi/browser';
import type { CommentComposerV2_user$key, CommentPopoverV2_thread$key, DocumentPanelV2CommentItem_thread$key } from '$mearie';

export type CommentAnchor = { rects: PageRect[] };

export type CommentThread = { id: string; selection: unknown } & DocumentPanelV2CommentItem_thread$key & CommentPopoverV2_thread$key;

export type CommentController = {
  readonly threads: CommentThread[];
  readonly resolvedThreads: CommentThread[];
  readonly showResolved: boolean;
  readonly activeThreadId: string | null;
  readonly activeThread: CommentThread | undefined;
  readonly activeAnchor: CommentAnchor | null;
  readonly composing: boolean;
  readonly myId: string;
  readonly isOwner: boolean;
  readonly meUser: CommentComposerV2_user$key;
  setShowResolved: (v: boolean) => void;
  isLocatable: (id: string) => boolean;
  openThread: (id: string, anchor?: CommentAnchor) => void;
  openFromPanel: (id: string) => void;
  close: () => void;
  createThread: (content: string) => Promise<void>;
  reply: (threadId: string, content: string) => Promise<void>;
  editComment: (commentId: string, content: string) => Promise<void>;
  deleteComment: (commentId: string) => Promise<void>;
  deleteThread: (threadId: string) => Promise<void>;
  resolveThread: (threadId: string) => Promise<void>;
  unresolveThread: (threadId: string) => Promise<void>;
};

const KEY = Symbol('DocumentComments');
export const setupCommentContext = (c: CommentController) => setContext(KEY, c);
export const getCommentContext = () => getContext<CommentController>(KEY);
