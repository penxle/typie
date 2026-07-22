package co.typie.screen.editor.editor.toolbar.contextual

import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalUriHandler
import co.typie.editor.DocumentEditingSession
import co.typie.editor.external.LocalEditorExternalElementState
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.PlainNode
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.icons.Lucide
import co.typie.platform.FilePickerResult
import co.typie.platform.FilePickerSelectionMode
import co.typie.platform.IncomingContentItem
import co.typie.platform.rememberFilePicker
import co.typie.screen.editor.editor.attachment.EditorAttachmentDestination
import co.typie.screen.editor.editor.attachment.LocalEditorAttachmentImporter
import co.typie.screen.editor.editor.toolbar.EditorToolbarButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarPage
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageKey
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageScope
import co.typie.screen.editor.editor.toolbar.EditorToolbarRow
import co.typie.ui.component.toast.LocalToast
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.launch

internal fun editorFileToolbarPage(
  file: PlainNode.File?,
  nodeId: String?,
  pickFile: (nodeId: String) -> Unit,
): EditorToolbarPage =
  EditorToolbarPage(
    key = EditorToolbarPageKey.File,
    icon = Lucide.Paperclip,
    contentDescription = "파일 툴바",
    ownerNodeId = nodeId,
    content = { scope ->
      EditorFileToolbar(scope = scope, file = file, nodeId = nodeId, pickFile = pickFile)
    },
  )

// Must be composed outside the toolbar's AnimatedVisibility: on Android the system picker hides
// the IME, which unmounts the toolbar pages, and an ActivityResult launcher registered there
// would be unregistered before the result arrives.
@Composable
internal fun rememberEditorFilePicker(): (nodeId: String) -> Unit {
  val toast = LocalToast.current
  val runtime = LocalEditorRuntime.current
  val importer = LocalEditorAttachmentImporter.current
  val scope = rememberCoroutineScope()
  val pendingRequest = remember { mutableStateOf<Pair<DocumentEditingSession, String>?>(null) }

  val picker =
    rememberFilePicker(selectionMode = FilePickerSelectionMode.Multiple) { result ->
      val request = pendingRequest.value
      pendingRequest.value = null
      val files =
        when (result) {
          FilePickerResult.Cancelled -> return@rememberFilePicker
          is FilePickerResult.Failed -> {
            toast.error("파일을 불러올 수 없어요.")
            return@rememberFilePicker
          }
          is FilePickerResult.Selected -> {
            if (result.unreadableCount > 0) {
              toast.error("일부 파일을 불러오지 못했어요.")
            }
            result.files
          }
        }
      if (request == null) {
        files.forEach { it.close() }
        return@rememberFilePicker
      }
      val (session, selectedNodeId) = request
      val requestedCount = files.size
      val items = files.map { file ->
        IncomingContentItem(kind = IncomingContentItem.Kind.File, file = file)
      }
      val onCompleted: (Int) -> Unit = { importedCount ->
        if (importedCount < requestedCount) {
          toast.error(
            if (importedCount == 0) {
              "파일을 삽입하지 못했어요."
            } else {
              "일부 파일을 삽입하지 못했어요."
            }
          )
        }
      }
      var importStarted = false
      scope
        .launch(start = CoroutineStart.UNDISPATCHED) {
          importStarted = true
          importer.import(
            session = session,
            items = items,
            destination =
              EditorAttachmentDestination.ExistingPlaceholder(
                nodeId = selectedNodeId,
                expectedKind = IncomingContentItem.Kind.File,
              ),
            onCompleted = onCompleted,
          )
        }
        .invokeOnCompletion {
          if (!importStarted) {
            items.forEach { item -> item.file.close() }
          }
        }
    }

  return remember(picker, runtime) {
    { nodeId ->
      val session = runtime.session
      if (session != null) {
        pendingRequest.value = session to nodeId
        picker("*/*")
      }
    }
  }
}

@Composable
private fun EditorFileToolbar(
  scope: EditorToolbarPageScope,
  file: PlainNode.File?,
  nodeId: String?,
  pickFile: (nodeId: String) -> Unit,
  modifier: Modifier = Modifier,
) {
  val uriHandler = LocalUriHandler.current
  val externalElementState = LocalEditorExternalElementState.current
  val fileState = externalElementState.files
  val fileId = file?.id
  val asset = fileId?.let(fileState.assets::get)
  val uploading = nodeId?.let { fileState.uploads.containsKey(it) } == true
  val hasFile = fileId != null || uploading

  EditorToolbarRow(scope = scope, modifier = modifier) {
    if (!hasFile) {
      EditorToolbarButton(
        icon = Lucide.Paperclip,
        contentDescription = "파일 첨부",
        onClick = { nodeId?.let(pickFile) },
      )
    }
    if (asset != null) {
      EditorToolbarButton(
        icon = Lucide.Download,
        contentDescription = "파일 다운로드",
        onClick = { uriHandler.openUri(asset.url) },
      )
    }
    EditorToolbarButton(
      icon = Lucide.Trash2,
      contentDescription = "파일 삭제",
      onClick = {
        val selectedNodeId = nodeId
        if (selectedNodeId != null) {
          fileState.uploads.remove(selectedNodeId)
          scope.sendMessage(Message.Node(NodeOp.Delete(id = selectedNodeId)))
        }
      },
    )
  }
}
