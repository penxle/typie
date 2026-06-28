package co.typie.screen.editor.editor.subpane.comments

import co.typie.graphql.fragment.CommentsSheetComment_comment
import co.typie.graphql.fragment.CommentsSheetThread_thread
import co.typie.graphql.type.UserRole

internal fun isRootComment(
  thread: CommentsSheetThread_thread,
  comment: CommentsSheetComment_comment,
): Boolean = thread.comments.firstOrNull()?.commentsSheetComment_comment?.id == comment.id

internal fun canUpdateComment(myId: String?, comment: CommentsSheetComment_comment): Boolean =
  myId != null && comment.user.commentsSheetUser_user.id == myId

internal fun canDeleteComment(
  myId: String?,
  myRole: UserRole?,
  isOwner: Boolean,
  thread: CommentsSheetThread_thread,
  comment: CommentsSheetComment_comment,
): Boolean {
  if (isRootComment(thread, comment)) {
    return canManageThread(myId = myId, myRole = myRole, isOwner = isOwner, thread = thread)
  }

  return myId != null &&
    (comment.user.commentsSheetUser_user.id == myId || isOwner || myRole == UserRole.ADMIN)
}

internal fun canManageThread(
  myId: String?,
  myRole: UserRole?,
  isOwner: Boolean,
  thread: CommentsSheetThread_thread,
): Boolean = myId != null && (thread.user.id == myId || isOwner || myRole == UserRole.ADMIN)
