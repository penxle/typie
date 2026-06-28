package co.typie.screen.editor.editor.subpane.comments

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.relocation.BringIntoViewRequester
import androidx.compose.foundation.relocation.bringIntoViewRequester
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.datetime.timeAgo
import co.typie.ext.clickable
import co.typie.graphql.fragment.CommentsSheetComment_comment
import co.typie.graphql.fragment.CommentsSheetThread_thread
import co.typie.graphql.fragment.CommentsSheetUser_user
import co.typie.graphql.type.UserRole
import co.typie.icons.Lucide
import co.typie.ui.component.Img
import co.typie.ui.component.Text
import co.typie.ui.component.popover.PopoverMenu
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppTheme

@Composable
internal fun CountBadge(count: Int) {
  Box(
    modifier =
      Modifier.clip(CircleShape)
        .background(AppTheme.colors.surfaceInset)
        .padding(horizontal = 7.dp, vertical = 2.dp),
    contentAlignment = Alignment.Center,
  ) {
    Text(
      text = count.toString(),
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textMuted,
    )
  }
}

@Composable
internal fun CommentThreadRow(
  thread: CommentsSheetThread_thread,
  filter: CommentFilter,
  active: Boolean,
  location: CommentThreadLocation?,
  myId: String?,
  myRole: UserRole?,
  isOwner: Boolean,
  editingCommentId: String?,
  editingText: String,
  replyText: String,
  onClick: () -> Unit,
  onStartEdit: (CommentsSheetComment_comment) -> Unit,
  onEditingTextChange: (String) -> Unit,
  onCancelEdit: () -> Unit,
  onSubmitEdit: (CommentsSheetComment_comment) -> Unit,
  onDeleteComment: (CommentsSheetComment_comment) -> Unit,
  onReplyTextChange: (String) -> Unit,
  onSubmitReply: (String) -> Unit,
  onInputFocusChanged: (Boolean) -> Unit,
  onResolve: () -> Unit,
  onUnresolve: () -> Unit,
  onDeleteThread: () -> Unit,
) {
  val bringIntoViewRequester = remember { BringIntoViewRequester() }
  val comments = thread.commentFragments
  val canManageThread =
    canManageThread(myId = myId, myRole = myRole, isOwner = isOwner, thread = thread)
  LaunchedEffect(active) {
    if (active) {
      bringIntoViewRequester.bringIntoView()
    }
  }

  Column(
    modifier =
      Modifier.fillMaxWidth()
        .bringIntoViewRequester(bringIntoViewRequester)
        .clip(RoundedCornerShape(8.dp))
        .background(if (active) AppTheme.colors.surfaceDefault else AppTheme.colors.surfaceInset)
        .border(
          width = 1.dp,
          color = if (active) AppTheme.colors.borderDefault else AppTheme.colors.borderHairline,
          shape = RoundedCornerShape(8.dp),
        )
        .then(if (!active) Modifier.clickable { onClick() } else Modifier)
        .padding(12.dp),
    verticalArrangement = Arrangement.spacedBy(12.dp),
  ) {
    CommentThreadHeader(
      location = location,
      commentCount = comments.size,
      canManage = canManageThread,
      filter = filter,
      onResolve = onResolve,
      onUnresolve = onUnresolve,
      onDeleteThread = onDeleteThread,
    )

    val visibleComments = if (active) comments else comments.take(1)
    visibleComments.forEachIndexed { index, comment ->
      if (index > 0) {
        CommentThreadDivider()
      }
      CommentItem(
        thread = thread,
        comment = comment,
        filter = filter,
        expanded = active,
        myId = myId,
        myRole = myRole,
        isOwner = isOwner,
        editing = active && editingCommentId == comment.id,
        editingText = editingText,
        onStartEdit = { onStartEdit(comment) },
        onEditingTextChange = onEditingTextChange,
        onCancelEdit = onCancelEdit,
        onSubmitEdit = { onSubmitEdit(comment) },
        onDeleteComment = { onDeleteComment(comment) },
        onInputFocusChanged = onInputFocusChanged,
      )
    }

    if (!active && comments.size > 1) {
      CommentThreadDivider()
      Text(
        text = "${comments.size - 1}개 더 보기",
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textMuted,
      )
    }

    if (active && filter == CommentFilter.Open && editingCommentId == null) {
      CommentThreadDivider()
      CommentComposer(
        value = replyText,
        onValueChange = onReplyTextChange,
        placeholder = "코멘트 추가...",
        onFocusChange = onInputFocusChanged,
        onSubmit = onSubmitReply,
      )
    }
  }
}

@Composable
internal fun VirtualCommentThreadRow(
  location: CommentThreadLocation?,
  value: String,
  onValueChange: (String) -> Unit,
  onFocusChange: (Boolean) -> Unit,
  onSubmit: (String) -> Unit,
) {
  val resolvedLocation = location ?: CommentThreadLocation.Missing
  Column(
    modifier =
      Modifier.fillMaxWidth()
        .clip(RoundedCornerShape(8.dp))
        .background(AppTheme.colors.surfaceDefault)
        .border(1.dp, AppTheme.colors.borderDefault, RoundedCornerShape(8.dp))
        .padding(12.dp),
    verticalArrangement = Arrangement.spacedBy(12.dp),
  ) {
    Text(
      text = commentExcerptText(location = resolvedLocation, virtual = true),
      style = AppTheme.typography.caption.copy(fontWeight = FontWeight.SemiBold),
      color = commentExcerptColor(location = resolvedLocation, virtual = true),
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )
    CommentThreadDivider()
    CommentComposer(
      value = value,
      onValueChange = onValueChange,
      placeholder = "코멘트 추가...",
      onFocusChange = onFocusChange,
      onSubmit = onSubmit,
    )
  }
}

@Composable
private fun CommentThreadHeader(
  location: CommentThreadLocation?,
  commentCount: Int,
  canManage: Boolean,
  filter: CommentFilter,
  onResolve: () -> Unit,
  onUnresolve: () -> Unit,
  onDeleteThread: () -> Unit,
) {
  Row(
    modifier = Modifier.fillMaxWidth(),
    horizontalArrangement = Arrangement.spacedBy(6.dp),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    if (filter == CommentFilter.Open) {
      if (location is CommentThreadLocation.Located) {
        Text(
          text = commentExcerptText(location = location),
          modifier = Modifier.weight(1f),
          style = AppTheme.typography.caption.copy(fontWeight = FontWeight.SemiBold),
          color = commentExcerptColor(location = location),
          maxLines = 1,
          overflow = TextOverflow.Ellipsis,
        )
      } else {
        Row(
          modifier = Modifier.weight(1f),
          horizontalArrangement = Arrangement.spacedBy(3.dp),
          verticalAlignment = Alignment.CenterVertically,
        ) {
          Icon(
            icon = Lucide.MapPinOff,
            modifier = Modifier.size(12.dp),
            tint = AppTheme.colors.textHint,
          )
          Text(
            text = "위치 없음",
            style = AppTheme.typography.caption.copy(fontWeight = FontWeight.SemiBold),
            color = AppTheme.colors.textHint,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
          )
        }
      }
    } else {
      Spacer(modifier = Modifier.weight(1f))
    }
    CountBadge(count = commentCount)

    if (canManage) {
      ThreadActionsMenu(
        filter = filter,
        onResolve = onResolve,
        onUnresolve = onUnresolve,
        onDeleteThread = onDeleteThread,
      )
    }
  }
}

@Composable
private fun CommentThreadDivider() {
  Box(Modifier.fillMaxWidth().height(1.dp).background(AppTheme.colors.borderHairline))
}

@Composable
private fun CommentItem(
  thread: CommentsSheetThread_thread,
  comment: CommentsSheetComment_comment,
  filter: CommentFilter,
  expanded: Boolean,
  myId: String?,
  myRole: UserRole?,
  isOwner: Boolean,
  editing: Boolean,
  editingText: String,
  onStartEdit: () -> Unit,
  onEditingTextChange: (String) -> Unit,
  onCancelEdit: () -> Unit,
  onSubmitEdit: () -> Unit,
  onDeleteComment: () -> Unit,
  onInputFocusChanged: (Boolean) -> Unit,
) {
  val actionsEnabled = filter == CommentFilter.Open && expanded
  val canEdit = actionsEnabled && canUpdateComment(myId = myId, comment = comment)
  val canDelete =
    actionsEnabled &&
      canDeleteComment(
        myId = myId,
        myRole = myRole,
        isOwner = isOwner,
        thread = thread,
        comment = comment,
      )

  Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.Top) {
    CommentAvatar(user = comment.user.commentsSheetUser_user)
    Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
      Row(
        modifier = Modifier.fillMaxWidth().height(28.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(6.dp),
      ) {
        Text(
          text = comment.user.commentsSheetUser_user.name,
          style = AppTheme.typography.caption.copy(fontWeight = FontWeight.SemiBold),
          color = AppTheme.colors.textDefault,
          maxLines = 1,
          overflow = TextOverflow.Ellipsis,
        )
        Text(
          text = comment.createdAt.timeAgo(),
          modifier = Modifier.weight(1f),
          style = AppTheme.typography.micro,
          color = AppTheme.colors.textHint,
          maxLines = 1,
          overflow = TextOverflow.Ellipsis,
        )
        if (!editing && (canEdit || canDelete)) {
          CommentActionsMenu(
            canEdit = canEdit,
            canDelete = canDelete,
            onStartEdit = onStartEdit,
            onDeleteComment = onDeleteComment,
          )
        }
      }

      if (editing) {
        Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
          CommentEditTextArea(
            value = editingText,
            onValueChange = onEditingTextChange,
            onFocusChange = onInputFocusChanged,
          )
          Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(14.dp, Alignment.End),
            verticalAlignment = Alignment.CenterVertically,
          ) {
            CommentTextActionButton(
              text = "취소",
              color = AppTheme.colors.textMuted,
              onClick = onCancelEdit,
            )
            CommentTextActionButton(
              text = "저장",
              color = AppTheme.colors.textDefault,
              onClick = onSubmitEdit,
            )
          }
        }
      } else {
        Text(
          text = comment.content,
          style = AppTheme.typography.body,
          color = AppTheme.colors.textDefault,
          maxLines = if (expanded) Int.MAX_VALUE else 3,
          overflow = TextOverflow.Ellipsis,
        )
      }
    }
  }
}

@Composable
private fun ThreadActionsMenu(
  filter: CommentFilter,
  onResolve: () -> Unit,
  onUnresolve: () -> Unit,
  onDeleteThread: () -> Unit,
) {
  PopoverMenu(anchor = { CommentActionAnchor(icon = Lucide.Ellipsis) }) {
    item(
      content = {
        CommentMenuRow(
          icon = if (filter == CommentFilter.Resolved) Lucide.Circle else Lucide.CircleCheck,
          text = if (filter == CommentFilter.Resolved) "다시 열기" else "해결",
        )
      },
      onClick = if (filter == CommentFilter.Resolved) onUnresolve else onResolve,
    )
    item(
      content = {
        CommentMenuRow(icon = Lucide.Trash2, text = "스레드 삭제", color = AppTheme.colors.danger)
      },
      onClick = onDeleteThread,
    )
  }
}

@Composable
private fun CommentActionsMenu(
  canEdit: Boolean,
  canDelete: Boolean,
  onStartEdit: () -> Unit,
  onDeleteComment: () -> Unit,
) {
  PopoverMenu(anchor = { CommentActionAnchor(icon = Lucide.Ellipsis) }) {
    if (canEdit) {
      item(content = { CommentMenuRow(icon = Lucide.Pencil, text = "수정") }, onClick = onStartEdit)
    }
    if (canDelete) {
      item(
        content = {
          CommentMenuRow(icon = Lucide.Trash2, text = "삭제", color = AppTheme.colors.danger)
        },
        onClick = onDeleteComment,
      )
    }
  }
}

@Composable
private fun CommentMenuRow(
  icon: IconData,
  text: String,
  color: Color = AppTheme.colors.textDefault,
) {
  Row(
    modifier = Modifier.height(42.dp).padding(horizontal = 16.dp),
    verticalAlignment = Alignment.CenterVertically,
    horizontalArrangement = Arrangement.spacedBy(12.dp),
  ) {
    Icon(icon = icon, modifier = Modifier.size(18.dp), tint = color)
    Text(text = text, style = AppTheme.typography.action, color = color)
  }
}

@Composable
private fun CommentActionAnchor(icon: IconData) {
  Box(modifier = Modifier.size(28.dp), contentAlignment = Alignment.Center) {
    Icon(icon = icon, modifier = Modifier.size(16.dp), tint = AppTheme.colors.textMuted)
  }
}

@Composable
private fun CommentAvatar(user: CommentsSheetUser_user) {
  Box(
    modifier = Modifier.size(24.dp).clip(CircleShape).background(AppTheme.colors.surfaceInset),
    contentAlignment = Alignment.Center,
  ) {
    Img(
      image = user.avatar.img_image,
      modifier = Modifier.fillMaxSize().clip(CircleShape),
      placeholderColor = AppTheme.colors.surfaceInset,
    )
  }
}

private fun commentExcerptText(location: CommentThreadLocation?, virtual: Boolean = false): String =
  when (location) {
    is CommentThreadLocation.Located -> location.excerpt.truncateExcerpt()
    CommentThreadLocation.Missing,
    null -> if (virtual) "선택한 텍스트" else "위치 없음"
  }

@Composable
private fun commentExcerptColor(location: CommentThreadLocation?, virtual: Boolean = false): Color =
  when (location) {
    is CommentThreadLocation.Located -> AppTheme.colors.textDefault
    CommentThreadLocation.Missing,
    null -> if (virtual) AppTheme.colors.textDefault else AppTheme.colors.textHint
  }

private fun String.truncateExcerpt(maxLength: Int = 80): String =
  if (length <= maxLength) this else "${take(maxLength).trimEnd()}…"

private val CommentsSheetThread_thread.commentFragments: List<CommentsSheetComment_comment>
  get() = comments.map { it.commentsSheetComment_comment }
