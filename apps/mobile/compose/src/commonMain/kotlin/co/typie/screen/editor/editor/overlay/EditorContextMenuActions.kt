package co.typie.screen.editor.editor.overlay

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalUriHandler
import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.PagePoint
import co.typie.editor.ext.isCollapsed
import co.typie.editor.ext.isSingleSlotRange
import co.typie.editor.external.EditorExternalImageElementState
import co.typie.editor.external.LocalEditorExternalElementState
import co.typie.editor.ffi.ClipboardOp
import co.typie.editor.ffi.ExternalElement
import co.typie.editor.ffi.ExternalElementData
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.SelectionExpansionUnit
import co.typie.editor.ffi.SelectionOp
import co.typie.editor.input.LocalEditorIncomingContentHandler
import co.typie.editor.runtime.EditorContextMenuState
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.scroll.EditorBringIntoViewPolicy
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.updateWithBringIntoView
import co.typie.icons.Lucide
import co.typie.platform.Clipboard
import co.typie.platform.IncomingContentMode
import co.typie.platform.PlatformModule
import co.typie.platform.rememberShareAnchor
import co.typie.screen.editor.editor.attachment.downloadEditorImage
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.icon.IconData

internal data class EditorContextMenuItem(
  val label: String,
  val icon: IconData,
  val onClick: () -> Unit,
  val destructive: Boolean = false,
  val modifier: Modifier = Modifier,
)

internal data class EditorContextMenuActions(
  val showCopyCutActions: Boolean,
  val availableExpansionUnits: Set<SelectionExpansionUnit>,
  val onCopy: () -> Unit,
  val onCut: () -> Unit,
  val onPaste: () -> Unit,
  val onExpandWord: () -> Unit,
  val onExpandSentence: () -> Unit,
  val onExpandParagraph: () -> Unit,
  val onSelectAll: () -> Unit,
  val onDismiss: () -> Unit,
  val contextualItems: List<EditorContextMenuItem> = emptyList(),
  val onComment: (() -> Unit)? = null,
)

@Composable
internal fun rememberEditorContextMenuActions(
  editor: Editor,
  bringIntoViewRequests: EditorBringIntoViewRequests,
  contextMenu: EditorContextMenuState,
  availableExpansionUnits: Set<SelectionExpansionUnit>,
  editorMutationEnabled: Boolean,
  onComment: (() -> Unit)?,
  clipboard: Clipboard = PlatformModule.clipboard,
): EditorContextMenuActions {
  val selection = editor.publishedState.selection
  val runtime = LocalEditorRuntime.current
  val incomingContentHandler = LocalEditorIncomingContentHandler.current
  val imageState = LocalEditorExternalElementState.current.images
  val image = editor.publishedState.contextMenuImage(contextMenu.pointerPosition, imageState)
  val imageAsset = (image?.data as? ExternalElementData.Image)?.id?.let(imageState.assets::get)
  val uriHandler = LocalUriHandler.current
  val toast = LocalToast.current
  val shareAnchor = rememberShareAnchor()
  return remember(
    editor,
    selection,
    availableExpansionUnits,
    bringIntoViewRequests,
    contextMenu,
    clipboard,
    runtime,
    incomingContentHandler,
    editorMutationEnabled,
    onComment,
    image,
    imageAsset,
    imageState,
    uriHandler,
    toast,
    shareAnchor,
  ) {
    val expandSelection =
      { unit: SelectionExpansionUnit, bringIntoViewTarget: EditorBringIntoViewTarget? ->
        val pointerPosition = contextMenu.pointerPosition
        val mode = contextMenu.mode
        editor.launchEffect {
          val appliedState =
            if (bringIntoViewTarget == null) {
              editor.update { enqueue(Message.Selection(SelectionOp.Expand(unit))) }?.snapshot
            } else {
              editor
                .updateWithBringIntoView(bringIntoViewRequests) {
                  enqueue(Message.Selection(SelectionOp.Expand(unit)))
                  bringIntoView(bringIntoViewTarget, policy = EditorBringIntoViewPolicy.CursorGuard)
                }
                ?.snapshot
            }
          appliedState?.let { state ->
            contextMenu.requestShowForAppliedSelection(
              editor = editor,
              state = state,
              pointerPosition = pointerPosition,
              mode = mode,
            )
          }
        }
      }

    EditorContextMenuActions(
      showCopyCutActions = !selection.isCollapsed(),
      availableExpansionUnits = availableExpansionUnits,
      onCopy = {
        editor.launchEffect {
          editor.copySelection()?.let { clipboard.copyRichText(html = it.html, text = it.text) }
        }
      },
      onCut = {
        editor.launchEffect {
          val payload = editor.copySelection() ?: return@launchEffect
          if (clipboard.copyRichText(html = payload.html, text = payload.text)) {
            editor.updateWithBringIntoView(bringIntoViewRequests) {
              enqueue(Message.Clipboard(ClipboardOp.Cut))
              bringIntoView(
                EditorBringIntoViewTarget.CurrentSelectionHead,
                policy = EditorBringIntoViewPolicy.CursorGuard,
              )
            }
          }
        }
      },
      onPaste = {
        val session = runtime.session?.takeIf { it.editor === editor }
        if (session != null) {
          editor.launchEffect {
            incomingContentHandler.handleClipboard(session, clipboard, IncomingContentMode.Rich)
          }
        }
      },
      onExpandWord = {
        expandSelection(SelectionExpansionUnit.Word, EditorBringIntoViewTarget.CurrentSelectionHead)
      },
      onExpandSentence = {
        expandSelection(
          SelectionExpansionUnit.Sentence,
          EditorBringIntoViewTarget.CurrentSelectionHead,
        )
      },
      onExpandParagraph = {
        expandSelection(
          SelectionExpansionUnit.Paragraph,
          EditorBringIntoViewTarget.CurrentSelectionHead,
        )
      },
      onSelectAll = { expandSelection(SelectionExpansionUnit.All, null) },
      onDismiss = contextMenu::hide,
      onComment = onComment,
      contextualItems =
        buildList {
          if (imageAsset != null) {
            add(
              EditorContextMenuItem(
                label = "이미지 내려받기",
                icon = Lucide.Download,
                modifier = shareAnchor.modifier,
                onClick = {
                  val anchor = shareAnchor.value
                  editor.launchEffect {
                    downloadEditorImage(imageAsset.originalUrl, anchor, uriHandler, toast)
                  }
                },
              )
            )
            add(
              EditorContextMenuItem(
                label = "원본 이미지 열기",
                icon = Lucide.ExternalLink,
                onClick = { uriHandler.openUri(imageAsset.originalUrl) },
              )
            )
          }
          if (image != null && editorMutationEnabled) {
            add(
              EditorContextMenuItem(
                label = "이미지 삭제",
                icon = Lucide.Trash2,
                destructive = true,
                onClick = {
                  imageState.uploads.remove(image.node)
                  editor.launchEffect {
                    editor.updateWithBringIntoView(bringIntoViewRequests) {
                      enqueue(Message.Node(NodeOp.Delete(image.node)))
                      bringIntoView(
                        EditorBringIntoViewTarget.CurrentSelectionHead,
                        policy = EditorBringIntoViewPolicy.CursorGuard,
                      )
                    }
                  }
                },
              )
            )
          }
        },
    )
  }
}

internal fun EditorState.contextMenuImage(
  pointer: PagePoint?,
  images: EditorExternalImageElementState,
): ExternalElement? = externalElements.firstOrNull { element ->
  val data = element.data as? ExternalElementData.Image ?: return@firstOrNull false
  if (pointer == null) {
    selection.isSingleSlotRange() && element.isSelected
  } else {
    val bounds = element.bounds
    val size = images.displaySize(element.node, data, bounds.width)
    val width = size?.width ?: bounds.width
    val height = size?.height ?: bounds.height
    val left = bounds.x + (bounds.width - width) / 2f
    element.pageIdx == pointer.page &&
      pointer.x >= left &&
      pointer.x <= left + width &&
      pointer.y >= bounds.y &&
      pointer.y <= bounds.y + height
  }
}
