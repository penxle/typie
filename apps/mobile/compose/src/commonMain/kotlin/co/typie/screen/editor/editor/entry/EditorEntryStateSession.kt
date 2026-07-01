package co.typie.screen.editor.editor.entry

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.snapshotFlow
import co.typie.editor.Editor
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionOp
import co.typie.editor.ffi.StableSelection
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.awaitWithBringIntoView
import kotlin.time.Clock
import kotlin.time.ExperimentalTime
import kotlinx.coroutines.flow.collect
import kotlinx.coroutines.flow.filterNotNull

@Stable
internal class EditorEntryStateSession(
  private val markElementFocused: (EditorEntryTarget) -> Unit
) {
  fun markTitleFocused() {
    markElementFocused(EditorEntryTarget.Title)
  }

  fun markSubtitleFocused() {
    markElementFocused(EditorEntryTarget.Subtitle)
  }
}

@Composable
internal fun rememberEditorEntryStateSession(
  documentId: String?,
  editor: Editor?,
  editorFocused: Boolean,
  bringIntoViewRequests: EditorBringIntoViewRequests,
): EditorEntryStateSession {
  val store = remember { EditorEntryStateStore() }
  val controller = remember { EditorEntryStateSessionController(store = store) }
  val currentEditorFocused = rememberUpdatedState(editorFocused)

  LaunchedEffect(documentId, editor, bringIntoViewRequests) {
    val activeDocumentId = documentId ?: return@LaunchedEffect
    val activeEditor = editor ?: return@LaunchedEffect
    val saved = store.load(activeDocumentId) ?: return@LaunchedEffect
    if (saved.target != EditorEntryTarget.Body) {
      return@LaunchedEffect
    }
    val selection = saved.bodySelection ?: return@LaunchedEffect

    activeEditor.awaitWithBringIntoView(bringIntoViewRequests) {
      enqueue(Message.Selection(SelectionOp.SetFrozen(selection = selection)))
      beforeCommit { bringIntoView(EditorBringIntoViewTarget.CurrentSelectionHead) }
    }
  }

  LaunchedEffect(documentId, editor) {
    val activeDocumentId = documentId ?: return@LaunchedEffect
    val activeEditor = editor ?: return@LaunchedEffect

    snapshotFlow { activeEditor.selection }
      .filterNotNull()
      .collect { selection ->
        if (!currentEditorFocused.value) {
          return@collect
        }

        controller.saveBodySelection(
          documentId = activeDocumentId,
          editor = activeEditor,
          selection = selection,
        )
      }
  }

  return remember(documentId, controller) {
    EditorEntryStateSession { target ->
      documentId?.let { activeDocumentId ->
        controller.saveElementFocus(documentId = activeDocumentId, target = target)
      }
    }
  }
}

private class EditorEntryStateSessionController(private val store: EditorEntryStateStore) {
  fun saveElementFocus(documentId: String, target: EditorEntryTarget) {
    save(documentId = documentId, target = target, bodySelection = null)
  }

  fun saveBodySelection(documentId: String, editor: Editor, selection: Selection) {
    val frozen = editor.freezeSelection(selection) ?: return
    save(documentId = documentId, target = EditorEntryTarget.Body, bodySelection = frozen)
  }

  @OptIn(ExperimentalTime::class)
  private fun save(documentId: String, target: EditorEntryTarget, bodySelection: StableSelection?) {
    store.save(
      documentId = documentId,
      state =
        StoredEditorEntryState(
          target = target,
          bodySelection = bodySelection,
          updatedAt = Clock.System.now().toEpochMilliseconds(),
        ),
    )
  }
}
