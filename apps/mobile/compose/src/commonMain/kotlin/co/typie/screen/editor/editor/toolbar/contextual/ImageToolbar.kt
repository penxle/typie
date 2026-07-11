package co.typie.screen.editor.editor.toolbar.contextual

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import co.typie.domain.blob.BlobService
import co.typie.editor.Editor
import co.typie.editor.external.EditorImageAsset
import co.typie.editor.external.EditorImageUpload
import co.typie.editor.external.LocalEditorExternalElementState
import co.typie.editor.ffi.ExternalElementData
import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.Fragment
import co.typie.editor.ffi.ImageNodeAttr
import co.typie.editor.ffi.InsertionOp
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeAttr
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
import co.typie.platform.FilePickerResult
import co.typie.platform.FilePickerSelectionMode
import co.typie.platform.PickedFile
import co.typie.platform.rememberFilePicker
import co.typie.screen.editor.editor.toEditorImageAsset
import co.typie.screen.editor.editor.toolbar.EditorToolbarButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarPage
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageKey
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageScope
import co.typie.screen.editor.editor.toolbar.EditorToolbarRow
import co.typie.screen.editor.editor.toolbar.EditorToolbarSecondary
import co.typie.ui.component.toast.LocalToast
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch

internal fun editorImageToolbarPage(image: PlainNode.Image?, nodeId: String?): EditorToolbarPage =
  EditorToolbarPage(
    key = EditorToolbarPageKey.Image,
    icon = Lucide.Image,
    contentDescription = "이미지 툴바",
    ownerNodeId = nodeId,
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
  val externalElementState = LocalEditorExternalElementState.current
  val imageState = externalElementState.images
  val imageId = image?.id
  val readyAsset = imageId?.let(imageState.assets::get)
  val uploading = nodeId?.let { imageState.uploads.containsKey(it) } == true
  val hasImage = imageId != null || uploading

  val picker =
    rememberFilePicker(selectionMode = FilePickerSelectionMode.Multiple) { result ->
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
      val selectedNodeId =
        nodeId
          ?: run {
            files.forEach { it.close() }
            return@rememberFilePicker
          }
      val imageUploads = files.mapNotNull { file ->
        val upload =
          file.toImageUploadOrNull()
            ?: run {
              file.close()
              return@mapNotNull null
            }
        file to upload
      }
      if (imageUploads.isEmpty()) {
        toast.error("이미지를 불러올 수 없어요.")
        return@rememberFilePicker
      }
      val skippedImageCount = files.size - imageUploads.size
      val (selectedImage, selectedUpload) = imageUploads.first()
      val restUploads = imageUploads.drop(1)

      val uploadJob =
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

          fun launchUpload(targetNodeId: String, file: PickedFile, upload: EditorImageUpload) {
            val job = launch {
              try {
                // TODO(TR-38): Check restrictedBlob before starting image uploads and
                // surface the
                // plan upgrade flow instead of letting the upload proceed.
                completeAttachmentOperation(
                  persist = { uploadImageAsset(file) },
                  isCurrent = { imageState.uploads[targetNodeId] === upload },
                  cache = externalElementState::put,
                  commit = { uploaded ->
                    val editor = checkNotNull(runtime.editor) { "No active editor is available" }
                    val committedState =
                      editor.awaitWithBringIntoView(bringIntoViewRequests) {
                        if (editor.ime?.composing != null) {
                          enqueue(Message.TextInput(listOf(FlatImeOp.CommitAsIs)))
                        }
                        enqueue(
                          Message.Node(
                            NodeOp.SetAttr(
                              id = targetNodeId,
                              attr = NodeAttr.Image(ImageNodeAttr.Id(uploaded.id)),
                            )
                          )
                        )
                        beforeCommit {
                          bringIntoView(EditorBringIntoViewTarget.CurrentSelectionHead)
                        }
                      }
                    checkNotNull(committedState) { "Editor image attrs did not commit" }
                  },
                  clearPending = {
                    if (imageState.uploads[targetNodeId] === upload) {
                      imageState.uploads.remove(targetNodeId)
                    }
                  },
                )
              } catch (error: CancellationException) {
                throw error
              } catch (error: Throwable) {
                reportAttachmentFailure(AttachmentKind.Image, error)
                toast.error("이미지 업로드에 실패했어요.")
              }
            }
            job.invokeOnCompletion { file.close() }
          }

          launchUpload(targetNodeId = selectedNodeId, file = selectedImage, upload = selectedUpload)
          restNodeIds.zip(restUploads).forEach { (newNodeId, pending) ->
            launchUpload(targetNodeId = newNodeId, file = pending.first, upload = pending.second)
          }
        }
      uploadJob.invokeOnCompletion { imageUploads.forEach { (file) -> file.close() } }
    }

  EditorToolbarRow(scope = scope, modifier = modifier) {
    if (!hasImage) {
      EditorToolbarButton(
        icon = Lucide.Image,
        contentDescription = "이미지 선택",
        onClick = { picker("image/*") },
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

  return EditorImageUpload(
    previewModel = previewModel,
    name = filename,
    width = width,
    height = height,
  )
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
    is ExternalElementData.Image -> element.node
    else -> null
  }
}

private suspend fun uploadImageAsset(file: PickedFile): EditorImageAsset {
  val path = BlobService.upload(file)
  val uploaded =
    Apollo.executeMutation(
        EditorScreen_PersistBlobAsImage_Mutation(input = PersistBlobAsImageInput(path = path))
      )
      .persistBlobAsImage
  return uploaded.toEditorImageAsset()
}
