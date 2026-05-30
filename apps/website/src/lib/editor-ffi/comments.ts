export type CommentReconcileResult = { toAdd: string[]; toRemove: string[] };

export function reconcileComments(registered: string[], desired: string[]): CommentReconcileResult {
  const reg = new Set(registered);
  const des = new Set(desired);
  return { toAdd: desired.filter((id) => !reg.has(id)), toRemove: registered.filter((id) => !des.has(id)) };
}

type ThreadLike = { user: { id: string } };
type ThreadWithComments = { comments: readonly { id: string; user?: { id: string } }[] };

export function isRootComment(thread: ThreadWithComments, commentId: string): boolean {
  return thread.comments.length > 0 && thread.comments[0].id === commentId;
}
export function canUpdateComment(comment: { user: { id: string } }, myId: string): boolean {
  return comment.user.id === myId;
}
export function canManageThread(thread: ThreadLike, myId: string, isOwner: boolean): boolean {
  return thread.user.id === myId || isOwner;
}
export function canDeleteComment(thread: ThreadWithComments, commentId: string, myId: string, isOwner: boolean): boolean {
  if (isRootComment(thread, commentId)) return false;
  const comment = thread.comments.find((c) => c.id === commentId);
  if (!comment?.user) return false;
  return comment.user.id === myId || isOwner;
}
