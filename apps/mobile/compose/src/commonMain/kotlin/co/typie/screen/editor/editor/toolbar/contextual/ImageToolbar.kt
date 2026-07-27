package co.typie.screen.editor.editor.toolbar.contextual

import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import co.typie.editor.external.EditorAssetResolution
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
import co.typie.screen.editor.editor.attachment.EditorAttachmentPickRequest
import co.typie.screen.editor.editor.attachment.LocalEditorAttachmentImporter
import co.typie.screen.editor.editor.toolbar.EditorToolbarButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarPage
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageKey
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageScope
import co.typie.screen.editor.editor.toolbar.EditorToolbarRow
import co.typie.screen.editor.editor.toolbar.EditorToolbarSecondary
import co.typie.ui.component.toast.LocalToast
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.launch

internal fun editorImageToolbarPage(
  image: PlainNode.Image?,
  nodeId: String?,
  pickImage: (nodeId: String, reclaimAssetId: String?) -> Unit,
): EditorToolbarPage =
  EditorToolbarPage(
    key = EditorToolbarPageKey.Image,
    icon = Lucide.Image,
    contentDescription = "이미지 툴바",
    ownerNodeId = nodeId,
    content = { scope ->
      EditorImageToolbar(scope = scope, image = image, nodeId = nodeId, pickImage = pickImage)
    },
  )

// Must be composed outside the toolbar's AnimatedVisibility: on Android the system picker hides
// the IME, which unmounts the toolbar pages, and an ActivityResult launcher registered there
// would be unregistered before the result arrives.
@Composable
internal fun rememberEditorImagePicker(): (nodeId: String, reclaimAssetId: String?) -> Unit {
  val toast = LocalToast.current
  val runtime = LocalEditorRuntime.current
  val importer = LocalEditorAttachmentImporter.current
  val scope = rememberCoroutineScope()
  val pendingRequest = remember { mutableStateOf<EditorAttachmentPickRequest?>(null) }

  val picker =
    rememberFilePicker(selectionMode = FilePickerSelectionMode.Multiple) { result ->
      val request = pendingRequest.value
      pendingRequest.value = null
      val files =
        when (result) {
          FilePickerResult.Cancelled -> return@rememberFilePicker
          is FilePickerResult.Failed -> {
            toast.error("이미지를 불러올 수 없어요.")
            return@rememberFilePicker
          }
          is FilePickerResult.Selected -> {
            if (result.unreadableCount > 0) {
              toast.error("일부 이미지를 불러오지 못했어요.")
            }
            result.files
          }
        }
      if (request == null) {
        files.forEach { it.close() }
        return@rememberFilePicker
      }
      val (session, selectedNodeId, reclaimAssetId) = request
      if (reclaimAssetId != null) {
        val reclaimed = files.first()
        files.drop(1).forEach { it.close() }
        importer.reselect(
          session = session,
          assetId = reclaimAssetId,
          nodeId = selectedNodeId,
          kind = IncomingContentItem.Kind.Image,
          file = reclaimed,
          onFailure = { toast.error("이미지를 삽입하지 못했어요.") },
        )
        return@rememberFilePicker
      }
      val requestedCount = files.size
      val items = files.map { file ->
        IncomingContentItem(kind = IncomingContentItem.Kind.Image, file = file)
      }
      val onCompleted: (Int) -> Unit = { importedCount ->
        if (importedCount < requestedCount) {
          toast.error(
            if (importedCount == 0) {
              "이미지를 삽입하지 못했어요."
            } else {
              "일부 이미지를 삽입하지 못했어요."
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
                expectedKind = IncomingContentItem.Kind.Image,
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
    { nodeId, reclaimAssetId ->
      val session = runtime.session
      if (session != null) {
        pendingRequest.value = EditorAttachmentPickRequest(session, nodeId, reclaimAssetId)
        picker("image/*")
      }
    }
  }
}

@Composable
private fun EditorImageToolbar(
  scope: EditorToolbarPageScope,
  image: PlainNode.Image?,
  nodeId: String?,
  pickImage: (nodeId: String, reclaimAssetId: String?) -> Unit,
  modifier: Modifier = Modifier,
) {
  val externalElementState = LocalEditorExternalElementState.current
  val imageState = externalElementState.images
  val importer = LocalEditorAttachmentImporter.current
  val imageId = image?.id
  val readyAsset = imageId?.let(imageState.assets::get)
  val uploading = nodeId?.let { imageState.uploads.containsKey(it) } == true
  // 서버가 missing으로 확정한 노드는 빈 placeholder와 같은 어포던스를 갖는다 — 그 ID를 그대로 재예약해
  // 이어받으므로 문서는 바뀌지 않는다.
  val unresolved =
    imageId != null &&
      readyAsset == null &&
      externalElementState.resolutions[imageId] == EditorAssetResolution.Missing

  EditorToolbarRow(scope = scope, modifier = modifier) {
    if (!uploading && (imageId == null || unresolved)) {
      EditorToolbarButton(
        icon = Lucide.Image,
        contentDescription = "이미지 선택",
        onClick = { nodeId?.let { pickImage(it, imageId) } },
      )
    }
    if (readyAsset != null && nodeId != null) {
      val resizeSecondary = EditorToolbarSecondary.ImageResize(nodeId = nodeId)
      val selected = scope.activeSecondaryToolbar == resizeSecondary
      EditorToolbarButton(
        icon = Lucide.MoveHorizontal,
        contentDescription = "이미지 폭 조정",
        selected = selected,
        onClick = { scope.toggleSecondaryToolbar(resizeSecondary) },
      )
    }
    EditorToolbarButton(
      icon = Lucide.Trash2,
      contentDescription = "이미지 삭제",
      onClick = {
        val selectedNodeId = nodeId
        if (selectedNodeId != null) {
          importer.cancelNode(selectedNodeId)
          scope.sendMessage(Message.Node(NodeOp.Delete(id = selectedNodeId)))
        }
      },
    )
  }
}
