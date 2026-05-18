package co.typie.screen.editor.editor.toolbar.contextual

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalUriHandler
import co.typie.domain.blob.BlobService
import co.typie.editor.external.EditorFileAsset
import co.typie.editor.external.EditorFileUpload
import co.typie.editor.external.LocalEditorExternalElementState
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.PlainNode
import co.typie.graphql.Apollo
import co.typie.graphql.EditorScreen_PersistBlobAsFile_Mutation
import co.typie.graphql.executeMutation
import co.typie.graphql.type.PersistBlobAsFileInput
import co.typie.icons.Lucide
import co.typie.platform.FilePickerSelectionMode
import co.typie.platform.PickedFile
import co.typie.platform.rememberFilePicker
import co.typie.screen.editor.editor.toolbar.EditorToolbarButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarPage
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageKey
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageScope
import co.typie.screen.editor.editor.toolbar.EditorToolbarRow
import co.typie.ui.component.toast.LocalToast
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch

internal fun editorFileToolbarPage(file: PlainNode.File?, nodeId: String?): EditorToolbarPage =
  EditorToolbarPage(
    key = EditorToolbarPageKey.File,
    icon = Lucide.Paperclip,
    contentDescription = "파일 툴바",
    content = { scope -> EditorFileToolbar(scope = scope, file = file, nodeId = nodeId) },
  )

@Composable
private fun EditorFileToolbar(
  scope: EditorToolbarPageScope,
  file: PlainNode.File?,
  nodeId: String?,
  modifier: Modifier = Modifier,
) {
  val toast = LocalToast.current
  val uriHandler = LocalUriHandler.current
  val fileState = LocalEditorExternalElementState.current.files
  val fileId = file?.id
  val asset = fileId?.let(fileState.assets::get)
  val uploading = nodeId?.let { fileState.uploads.containsKey(it) } == true
  val hasFile = fileId != null || uploading

  val picker =
    rememberFilePicker(selectionMode = FilePickerSelectionMode.Single) { files ->
      val selectedNodeId = nodeId ?: return@rememberFilePicker
      val selectedFile = files.firstOrNull() ?: return@rememberFilePicker

      scope.commandScope.launch {
        val upload =
          EditorFileUpload(name = selectedFile.filename, size = selectedFile.bytes.size.toLong())
        fileState.uploads[selectedNodeId] = upload
        try {
          // TODO(TR-38): Check restrictedBlob before starting file uploads and surface the
          // plan upgrade flow instead of letting the upload proceed.
          val uploaded = uploadFileAsset(selectedFile)
          if (fileState.uploads[selectedNodeId] !== upload) {
            return@launch
          }
          fileState.assets[uploaded.id] = uploaded
          scope.sendMessage(
            Message.Node(
              NodeOp.SetAttrs(id = selectedNodeId, attrs = PlainNode.File(id = uploaded.id))
            )
          )
        } catch (error: CancellationException) {
          throw error
        } catch (_: Throwable) {
          toast.error("파일 업로드에 실패했어요.")
        } finally {
          if (fileState.uploads[selectedNodeId] === upload) {
            fileState.uploads.remove(selectedNodeId)
          }
        }
      }
    }

  EditorToolbarRow(scope = scope, modifier = modifier) {
    if (!hasFile) {
      EditorToolbarButton(
        icon = Lucide.Paperclip,
        contentDescription = "파일 첨부",
        onClick = { picker("*/*") },
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

private suspend fun uploadFileAsset(file: PickedFile): EditorFileAsset {
  val path = BlobService.uploadBytes(file.bytes, filename = file.filename, mimeType = file.mimeType)
  val uploaded =
    Apollo.executeMutation(
        EditorScreen_PersistBlobAsFile_Mutation(input = PersistBlobAsFileInput(path = path))
      )
      .persistBlobAsFile
  return EditorFileAsset(
    id = uploaded.id,
    name = uploaded.name,
    url = uploaded.url,
    size = uploaded.size,
  )
}
