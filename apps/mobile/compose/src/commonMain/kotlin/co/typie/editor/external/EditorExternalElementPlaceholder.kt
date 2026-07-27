package co.typie.editor.external

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.style.TextOverflow
import co.touchlab.kermit.Logger
import co.typie.editor.DocumentEditingSession
import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeOp
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.ext.clickable
import co.typie.icons.Lucide
import co.typie.platform.IncomingContentItem
import co.typie.screen.editor.editor.attachment.LocalEditorAttachmentImporter
import co.typie.screen.editor.editor.attachment.LocalEditorAttachmentPicker
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

@Composable
context(scope: EditorExternalElementRenderScope)
internal fun EditorExternalElementPlaceholder(
  icon: IconData,
  text: String,
  trailing: (@Composable () -> Unit)? = null,
) {
  Row(
    modifier =
      Modifier.height(scope.scaledDp(48f))
        .fillMaxWidth()
        .clip(scope.shape)
        .background(AppTheme.colors.surfaceInset, scope.shape)
        .padding(horizontal = scope.scaledDp(14f), vertical = scope.scaledDp(12f)),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Icon(
      icon = icon,
      contentDescription = null,
      modifier = Modifier.size(scope.scaledDp(20f)),
      tint = AppTheme.colors.textHint,
    )
    Text(
      text = text,
      modifier = Modifier.padding(start = scope.scaledDp(12f)).weight(1f),
      color = AppTheme.colors.textHint,
      style = AppTheme.typography.body.copy(fontSize = scope.scaledSp(14f)),
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )
    trailing?.invoke()
  }
}

@Composable
context(scope: EditorExternalElementRenderScope)
internal fun EditorExternalElementFailureCard(
  icon: IconData,
  text: String,
  deleteContentDescription: String,
  onRetry: (() -> Unit)?,
  onReselect: () -> Unit,
  onDelete: () -> Unit,
) {
  Row(
    modifier =
      Modifier.height(scope.scaledDp(48f))
        .fillMaxWidth()
        .clip(scope.shape)
        .background(AppTheme.colors.surfaceInset, scope.shape)
        .padding(start = scope.scaledDp(14f), end = scope.scaledDp(8f)),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Icon(
      icon = icon,
      contentDescription = null,
      modifier = Modifier.size(scope.scaledDp(20f)),
      tint = AppTheme.colors.textHint,
    )
    Text(
      text = text,
      modifier = Modifier.padding(start = scope.scaledDp(12f)).weight(1f),
      color = AppTheme.colors.textHint,
      style = AppTheme.typography.body.copy(fontSize = scope.scaledSp(14f)),
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )
    if (onRetry != null) {
      FailureAction(text = "재시도", onClick = onRetry)
    }
    FailureAction(text = "다시 선택", onClick = onReselect)
    Box(
      modifier =
        Modifier.clip(AppShapes.rounded(scope.scaledDp(4f)))
          .clickable(onClick = onDelete)
          .padding(scope.scaledDp(6f))
    ) {
      Icon(
        icon = Lucide.Trash2,
        contentDescription = deleteContentDescription,
        modifier = Modifier.size(scope.scaledDp(16f)),
        tint = AppTheme.colors.textHint,
      )
    }
  }
}

@Composable
context(scope: EditorExternalElementRenderScope)
private fun FailureAction(text: String, onClick: () -> Unit) {
  Text(
    text = text,
    modifier =
      Modifier.clip(AppShapes.rounded(scope.scaledDp(4f)))
        .clickable(onClick = onClick)
        .padding(horizontal = scope.scaledDp(8f), vertical = scope.scaledDp(4f)),
    color = AppTheme.colors.textMuted,
    style = AppTheme.typography.body.copy(fontSize = scope.scaledSp(13f)),
    maxLines = 1,
  )
}

@Composable
context(scope: EditorExternalElementRenderScope)
internal fun EditorAttachmentFailureCard(
  icon: IconData,
  text: String,
  deleteContentDescription: String,
  kind: IncomingContentItem.Kind,
  assetId: String?,
  nodeId: String,
) {
  val runtime = LocalEditorRuntime.current
  val importer = LocalEditorAttachmentImporter.current
  val picker = LocalEditorAttachmentPicker.current

  EditorExternalElementFailureCard(
    icon = icon,
    text = text,
    deleteContentDescription = deleteContentDescription,
    onRetry =
      if (assetId != null && importer.canRetry(assetId)) {
        { importer.retry(assetId) }
      } else {
        null
      },
    onReselect = { picker.pick(kind, nodeId, assetId) },
    onDelete = {
      importer.cancelNode(nodeId)
      deleteExternalElementNode(runtime.session, nodeId)
    },
  )
}

private fun deleteExternalElementNode(session: DocumentEditingSession?, nodeId: String) {
  if (session == null) {
    Logger.e { "External element delete was dropped: no editing session is attached" }
    return
  }
  val editor = session.editor
  val command = session.submit { _, context ->
    editor.scope.launch(context) {
      val committed = editor.await {
        if (editor.ime?.composing != null) {
          enqueue(Message.TextInput(listOf(FlatImeOp.CommitAsIs)))
        }
        enqueue(Message.Node(NodeOp.Delete(id = nodeId)))
      }
      if (!committed) {
        Logger.e { "External element delete did not commit" }
      }
    }
  }
  if (command == null) {
    Logger.e { "External element delete was dropped: the editing session is no longer active" }
  }
}
