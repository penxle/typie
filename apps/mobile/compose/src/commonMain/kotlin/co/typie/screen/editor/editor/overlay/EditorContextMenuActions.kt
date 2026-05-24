package co.typie.screen.editor.editor.overlay

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import co.typie.editor.Editor
import co.typie.editor.ext.isCollapsed
import co.typie.editor.ffi.ClipboardOp
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.SelectionExpansionUnit
import co.typie.editor.ffi.SelectionOp
import co.typie.editor.runtime.EditorContextMenuState
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.awaitWithBringIntoView
import co.typie.platform.Clipboard
import co.typie.platform.PlatformModule
import kotlinx.coroutines.launch

internal data class EditorContextMenuActions(
  val showCopyCutActions: Boolean,
  val onCopy: () -> Unit,
  val onCut: () -> Unit,
  val onPaste: () -> Unit,
  val onExpandWord: () -> Unit,
  val onExpandSentence: () -> Unit,
  val onExpandParagraph: () -> Unit,
  val onSelectAll: () -> Unit,
  val onDismiss: () -> Unit,
)

@Composable
internal fun rememberEditorContextMenuActions(
  editor: Editor,
  bringIntoViewRequests: EditorBringIntoViewRequests,
  contextMenu: EditorContextMenuState,
  clipboard: Clipboard = PlatformModule.clipboard,
): EditorContextMenuActions {
  val coroutineScope = rememberCoroutineScope()
  val selection = editor.selection
  return remember(
    editor,
    selection,
    bringIntoViewRequests,
    contextMenu,
    clipboard,
    coroutineScope,
  ) {
    val expandSelection =
      { unit: SelectionExpansionUnit, bringIntoViewTarget: EditorBringIntoViewTarget? ->
        contextMenu.requestShowAfterSelectionCommit()
        coroutineScope.launch {
          if (bringIntoViewTarget == null) {
            editor.await { enqueue(Message.Selection(SelectionOp.Expand(unit))) }
          } else {
            editor.awaitWithBringIntoView(bringIntoViewRequests) {
              enqueue(Message.Selection(SelectionOp.Expand(unit)))
              beforeCommit { bringIntoView(bringIntoViewTarget) }
            }
          }
        }
      }

    EditorContextMenuActions(
      showCopyCutActions = !selection.isCollapsed(),
      onCopy = {
        coroutineScope.launch {
          editor.copySelection()?.let { clipboard.copyRichText(html = it.html, text = it.text) }
        }
      },
      onCut = {
        coroutineScope.launch {
          val payload = editor.copySelection() ?: return@launch
          if (clipboard.copyRichText(html = payload.html, text = payload.text)) {
            editor.awaitWithBringIntoView(bringIntoViewRequests) {
              enqueue(Message.Clipboard(ClipboardOp.Cut))
              beforeCommit { bringIntoView(EditorBringIntoViewTarget.CurrentSelectionHead) }
            }
          }
        }
      },
      onPaste = {
        coroutineScope.launch {
          val read = clipboard.paste() ?: return@launch
          editor.awaitWithBringIntoView(bringIntoViewRequests) {
            enqueue(Message.Clipboard(ClipboardOp.Paste(html = read.html, text = read.text)))
            beforeCommit { bringIntoView(EditorBringIntoViewTarget.CurrentCursorLine) }
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
    )
  }
}
