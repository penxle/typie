package co.typie.screen.editor.editor.toolbar.contextual

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import co.typie.domain.blob.BlobService
import co.typie.editor.Editor
import co.typie.editor.external.EditorImageAsset
import co.typie.editor.external.EditorImageUpload
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
import co.typie.graphql.EditorScreen_PersistBlobAsImage_Mutation
import co.typie.graphql.executeMutation
import co.typie.graphql.type.PersistBlobAsImageInput
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

internal fun editorImageToolbarPage(image: PlainNode.Image?, nodeId: String?): EditorToolbarPage =
  EditorToolbarPage(
    key = EditorToolbarPageKey.Image,
    icon = Lucide.Image,
    contentDescription = "이미지 툴바",
    content = { scope -> EditorImageToolbar(scope = scope, image = image, nodeId = nodeId) },
  )

@Composable
private fun EditorImageToolbar(
  scope: EditorToolbarPageScope,
  image: PlainNode.Image?,
  nodeId: String?,
  modifier: Modifier = Modifier,
) {
  val toast = LocalToast.current
  val runtime = LocalEditorRuntime.current
  val bringIntoViewRequests = LocalEditorBringIntoViewRequests.current
  val imageState = LocalEditorExternalElementState.current.images
  val imageId = image?.id
  val uploading = nodeId?.let { imageState.uploads.containsKey(it) } == true
  val hasImage = imageId != null || uploading

  val picker =
    rememberFilePicker(selectionMode = FilePickerSelectionMode.Multiple) { files ->
      val selectedNodeId = nodeId ?: return@rememberFilePicker
      if (files.isEmpty()) {
        return@rememberFilePicker
      }
      val imageUploads = files.mapNotNull { file ->
        val upload = file.toImageUploadOrNull() ?: return@mapNotNull null
        file to upload
      }
      if (imageUploads.isEmpty()) {
        toast.error("이미지를 불러올 수 없어요.")
        return@rememberFilePicker
      }
      val skippedImageCount = files.size - imageUploads.size
      val (selectedImage, selectedUpload) = imageUploads.first()
      val restUploads = imageUploads.drop(1)

      scope.commandScope.launch {
        imageState.uploads[selectedNodeId] = selectedUpload

        val restNodeIds =
          try {
            insertImagePlaceholders(
              editor = runtime.editor,
              bringIntoViewRequests = bringIntoViewRequests,
              count = restUploads.size,
            )
          } catch (error: CancellationException) {
            throw error
          } catch (_: Throwable) {
            toast.error("이미지 업로드에 실패했어요.")
            emptyList()
          }

        if (skippedImageCount > 0 || restNodeIds.size < restUploads.size) {
          toast.error("일부 이미지를 삽입하지 못했어요.")
        }

        restNodeIds.zip(restUploads).forEach { (newNodeId, pending) ->
          imageState.uploads[newNodeId] = pending.second
        }

        fun launchUpload(
          targetNodeId: String,
          file: PickedFile,
          upload: EditorImageUpload,
          proportion: Int,
        ) {
          launch {
            try {
              // TODO(TR-38): Check restrictedBlob before starting image uploads and surface the
              // plan upgrade flow instead of letting the upload proceed.
              val uploaded = uploadImageAsset(file)
              if (imageState.uploads[targetNodeId] !== upload) {
                return@launch
              }
              imageState.assets[uploaded.id] = uploaded
              scope.sendMessage(
                Message.Node(
                  NodeOp.SetAttrs(
                    id = targetNodeId,
                    attrs = PlainNode.Image(id = uploaded.id, proportion = proportion),
                  )
                )
              )
            } catch (error: CancellationException) {
              throw error
            } catch (_: Throwable) {
              toast.error("이미지 업로드에 실패했어요.")
            } finally {
              if (imageState.uploads[targetNodeId] === upload) {
                imageState.uploads.remove(targetNodeId)
              }
            }
          }
        }

        launchUpload(
          targetNodeId = selectedNodeId,
          file = selectedImage,
          upload = selectedUpload,
          proportion = image?.proportion ?: 100,
        )
        restNodeIds.zip(restUploads).forEach { (newNodeId, pending) ->
          launchUpload(
            targetNodeId = newNodeId,
            file = pending.first,
            upload = pending.second,
            proportion = 100,
          )
        }
      }
    }

  EditorToolbarRow(scope = scope, modifier = modifier) {
    if (!hasImage) {
      EditorToolbarButton(
        icon = Lucide.Image,
        contentDescription = "이미지 선택",
        onClick = { picker("image/*") },
      )
    }
    EditorToolbarButton(
      icon = Lucide.Trash2,
      contentDescription = "이미지 삭제",
      onClick = {
        val selectedNodeId = nodeId
        if (selectedNodeId != null) {
          imageState.uploads.remove(selectedNodeId)
          scope.sendMessage(Message.Node(NodeOp.Delete(id = selectedNodeId)))
        }
      },
    )
  }
}

private fun PickedFile.toImageUploadOrNull(): EditorImageUpload? {
  val width = imageWidth
  val height = imageHeight
  if (width == null || height == null || width <= 0 || height <= 0) {
    return null
  }

  return EditorImageUpload(bytes = bytes, name = filename, width = width, height = height)
}

private suspend fun insertImagePlaceholders(
  editor: Editor?,
  bringIntoViewRequests: EditorBringIntoViewRequests,
  count: Int,
): List<String> {
  if (editor == null || count <= 0) {
    return emptyList()
  }

  val beforeNodeIds = editor.imageExternalNodeIds().toSet()
  editor.awaitWithBringIntoView(bringIntoViewRequests) {
    repeat(count) {
      enqueue(Message.Insertion(InsertionOp.Fragment(Fragment(node = PlainNode.Image(id = null)))))
    }
    beforeCommit { bringIntoView(EditorBringIntoViewTarget.CurrentSelectionHead) }
  }

  return editor.imageExternalNodeIds().filterNot(beforeNodeIds::contains)
}

private fun Editor.imageExternalNodeIds(): List<String> = externalElements.mapNotNull { element ->
  when (element.data) {
    is ExternalElementData.Image -> element.nodeId
    else -> null
  }
}

private suspend fun uploadImageAsset(file: PickedFile): EditorImageAsset {
  val path = BlobService.uploadBytes(file.bytes, filename = file.filename, mimeType = file.mimeType)
  val uploaded =
    Apollo.executeMutation(
        EditorScreen_PersistBlobAsImage_Mutation(input = PersistBlobAsImageInput(path = path))
      )
      .persistBlobAsImage
  return EditorImageAsset(
    id = uploaded.id,
    url = uploaded.url,
    width = uploaded.width,
    height = uploaded.height,
    ratio = uploaded.ratio,
    placeholder = uploaded.placeholder,
  )
}
