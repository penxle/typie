package co.typie.screen.editor.editor.toolbar.contextual

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import co.typie.domain.blob.BlobService
import co.typie.editor.external.EditorImageAsset
import co.typie.editor.external.EditorImageUpload
import co.typie.editor.external.LocalEditorExternalElementState
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.PlainNode
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
  val imageState = LocalEditorExternalElementState.current.images
  val imageId = image?.id
  val uploading = nodeId?.let { imageState.uploads.containsKey(it) } == true
  val hasImage = imageId != null || uploading

  val picker =
    rememberFilePicker(selectionMode = FilePickerSelectionMode.Single) { files ->
      val selectedNodeId = nodeId ?: return@rememberFilePicker
      val selectedImage = files.firstOrNull() ?: return@rememberFilePicker
      val width = selectedImage.imageWidth
      val height = selectedImage.imageHeight
      if (width == null || height == null || width <= 0 || height <= 0) {
        toast.error("이미지를 불러올 수 없어요.")
        return@rememberFilePicker
      }

      scope.commandScope.launch {
        val upload =
          EditorImageUpload(
            bytes = selectedImage.bytes,
            name = selectedImage.filename,
            width = width,
            height = height,
          )
        imageState.uploads[selectedNodeId] = upload
        try {
          // TODO(TR-38): Check restrictedBlob before starting image uploads and surface the
          // plan upgrade flow instead of letting the upload proceed.
          val uploaded = uploadImageAsset(selectedImage)
          if (imageState.uploads[selectedNodeId] !== upload) {
            return@launch
          }
          imageState.assets[uploaded.id] = uploaded
          scope.sendMessage(
            Message.Node(
              NodeOp.SetAttrs(
                id = selectedNodeId,
                attrs = PlainNode.Image(id = uploaded.id, proportion = image?.proportion ?: 100),
              )
            )
          )
        } catch (error: CancellationException) {
          throw error
        } catch (_: Throwable) {
          toast.error("이미지 업로드에 실패했어요.")
        } finally {
          if (imageState.uploads[selectedNodeId] === upload) {
            imageState.uploads.remove(selectedNodeId)
          }
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
