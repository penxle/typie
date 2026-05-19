package co.typie.screen.editor.editor.toolbar.contextual

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalUriHandler
import co.typie.domain.blob.BlobService
import co.typie.editor.Editor
import co.typie.editor.external.EditorFileAsset
import co.typie.editor.external.EditorFileUpload
import co.typie.editor.external.LocalEditorExternalElementState
import co.typie.editor.ffi.ExternalElementData
import co.typie.editor.ffi.Fragment
import co.typie.editor.ffi.InsertionOp
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.PlainNode
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.LocalEditorBringIntoViewRequests
import co.typie.editor.scroll.awaitWithBringIntoView
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
  val runtime = LocalEditorRuntime.current
  val bringIntoViewRequests = LocalEditorBringIntoViewRequests.current
  val uriHandler = LocalUriHandler.current
  val fileState = LocalEditorExternalElementState.current.files
  val fileId = file?.id
  val asset = fileId?.let(fileState.assets::get)
  val uploading = nodeId?.let { fileState.uploads.containsKey(it) } == true
  val hasFile = fileId != null || uploading

  val picker =
    rememberFilePicker(selectionMode = FilePickerSelectionMode.Multiple) { files ->
      val selectedNodeId = nodeId ?: return@rememberFilePicker
      val selectedFile = files.firstOrNull() ?: return@rememberFilePicker
      val selectedUpload = selectedFile.toFileUpload()
      val restUploads = files.drop(1).map { file -> file to file.toFileUpload() }

      scope.commandScope.launch {
        fileState.uploads[selectedNodeId] = selectedUpload

        val restNodeIds =
          try {
            insertFilePlaceholders(
              editor = runtime.editor,
              bringIntoViewRequests = bringIntoViewRequests,
              count = restUploads.size,
            )
          } catch (error: CancellationException) {
            throw error
          } catch (_: Throwable) {
            toast.error("파일 업로드에 실패했어요.")
            emptyList()
          }

        if (restNodeIds.size < restUploads.size) {
          toast.error("일부 파일을 삽입하지 못했어요.")
        }

        restNodeIds.zip(restUploads).forEach { (newNodeId, pending) ->
          fileState.uploads[newNodeId] = pending.second
        }

        fun launchUpload(targetNodeId: String, file: PickedFile, upload: EditorFileUpload) {
          launch {
            try {
              // TODO(TR-38): Check restrictedBlob before starting file uploads and surface the
              // plan upgrade flow instead of letting the upload proceed.
              val uploaded = uploadFileAsset(file)
              if (fileState.uploads[targetNodeId] !== upload) {
                return@launch
              }
              fileState.assets[uploaded.id] = uploaded
              scope.sendMessage(
                Message.Node(
                  NodeOp.SetAttrs(id = targetNodeId, attrs = PlainNode.File(id = uploaded.id))
                )
              )
            } catch (error: CancellationException) {
              throw error
            } catch (_: Throwable) {
              toast.error("파일 업로드에 실패했어요.")
            } finally {
              if (fileState.uploads[targetNodeId] === upload) {
                fileState.uploads.remove(targetNodeId)
              }
            }
          }
        }

        launchUpload(targetNodeId = selectedNodeId, file = selectedFile, upload = selectedUpload)
        restNodeIds.zip(restUploads).forEach { (newNodeId, pending) ->
          launchUpload(targetNodeId = newNodeId, file = pending.first, upload = pending.second)
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

private fun PickedFile.toFileUpload(): EditorFileUpload =
  EditorFileUpload(name = filename, size = bytes.size.toLong())

private suspend fun insertFilePlaceholders(
  editor: Editor?,
  bringIntoViewRequests: EditorBringIntoViewRequests,
  count: Int,
): List<String> {
  if (editor == null || count <= 0) {
    return emptyList()
  }

  val beforeNodeIds = editor.fileExternalNodeIds().toSet()
  editor.awaitWithBringIntoView(bringIntoViewRequests) {
    repeat(count) {
      enqueue(Message.Insertion(InsertionOp.Fragment(Fragment(node = PlainNode.File(id = null)))))
    }
    beforeCommit { bringIntoView(EditorBringIntoViewTarget.CurrentSelectionHead) }
  }

  return editor.fileExternalNodeIds().filterNot(beforeNodeIds::contains)
}

private fun Editor.fileExternalNodeIds(): List<String> = externalElements.mapNotNull { element ->
  when (element.data) {
    is ExternalElementData.File -> element.nodeId
    else -> null
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
