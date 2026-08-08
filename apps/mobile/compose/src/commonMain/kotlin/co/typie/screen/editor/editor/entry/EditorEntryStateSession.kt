package co.typie.screen.editor.editor.entry

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import co.typie.editor.Editor
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.SelectionOp
import co.typie.editor.ffi.StableSelection
import co.typie.editor.scroll.EditorBringIntoViewPolicy
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.updateWithBringIntoView
import kotlin.time.Clock
import kotlin.time.ExperimentalTime
import kotlinx.coroutines.flow.collect

@Stable
internal class EditorEntryStateSession(
  initialPresentationReady: Boolean,
  private val markElementFocused: (EditorEntryTarget) -> Unit,
) {
  var presentationReady: Boolean by mutableStateOf(initialPresentationReady)
    private set

  fun markTitleFocused() {
    markElementFocused(EditorEntryTarget.Title)
  }

  fun markSubtitleFocused() {
    markElementFocused(EditorEntryTarget.Subtitle)
  }

  internal fun markPresentationReady() {
    presentationReady = true
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
  val saved = remember(documentId) { documentId?.let(store::load) }
  val restoreSelection = saved?.bodySelection?.takeIf { saved.target == EditorEntryTarget.Body }
  val session =
    remember(documentId, editor, controller, restoreSelection) {
      EditorEntryStateSession(initialPresentationReady = restoreSelection == null) { target ->
        documentId?.let { activeDocumentId ->
          controller.saveElementFocus(documentId = activeDocumentId, target = target)
        }
      }
    }

  LaunchedEffect(documentId, editor, bringIntoViewRequests, restoreSelection, session) {
    documentId ?: return@LaunchedEffect
    val activeEditor = editor ?: return@LaunchedEffect
    val selection = restoreSelection ?: return@LaunchedEffect

    activeEditor.updateWithBringIntoView(bringIntoViewRequests) {
      enqueue(Message.Selection(SelectionOp.SetFrozen(selection = selection)))
      bringIntoView(
        EditorBringIntoViewTarget.CurrentSelectionHead,
        policy = EditorBringIntoViewPolicy.CursorGuard,
      )
    }
    session.markPresentationReady()
  }

  LaunchedEffect(documentId, editor) {
    val activeDocumentId = documentId ?: return@LaunchedEffect
    val activeEditor = editor ?: return@LaunchedEffect
    var wasFocused = currentEditorFocused.value
    var baselineSelection =
      if (wasFocused) {
        null
      } else {
        activeEditor.appliedState.selection
      }

    snapshotFlow { currentEditorFocused.value to activeEditor.appliedState.selection }
      .collect { (focused, selection) ->
        if (focused && selection != null && selection != baselineSelection) {
          controller.saveBodySelection(documentId = activeDocumentId, editor = activeEditor)
        }

        if (focused || wasFocused) {
          baselineSelection = selection
        }
        wasFocused = focused
      }
  }

  return session
}

private class EditorEntryStateSessionController(private val store: EditorEntryStateStore) {
  fun saveElementFocus(documentId: String, target: EditorEntryTarget) {
    save(documentId = documentId, target = target, bodySelection = null)
  }

  suspend fun saveBodySelection(documentId: String, editor: Editor) {
    val selection = editor.appliedState.selection ?: return
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
