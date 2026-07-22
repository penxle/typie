package co.typie.editor.input

import androidx.compose.runtime.compositionLocalOf
import co.typie.editor.DocumentEditingSession
import co.typie.platform.Clipboard
import co.typie.platform.IncomingContentCandidates
import co.typie.platform.IncomingContentMode

internal interface EditorIncomingContentHandler {
  suspend fun handleClipboard(
    session: DocumentEditingSession,
    clipboard: Clipboard,
    mode: IncomingContentMode,
  ): Boolean

  suspend fun handleCandidates(
    session: DocumentEditingSession,
    candidates: IncomingContentCandidates,
    mode: IncomingContentMode = IncomingContentMode.Rich,
  ): Boolean
}

internal val LocalEditorIncomingContentHandler =
  compositionLocalOf<EditorIncomingContentHandler> { NoopEditorIncomingContentHandler }

internal object NoopEditorIncomingContentHandler : EditorIncomingContentHandler {
  override suspend fun handleClipboard(
    session: DocumentEditingSession,
    clipboard: Clipboard,
    mode: IncomingContentMode,
  ): Boolean = false

  override suspend fun handleCandidates(
    session: DocumentEditingSession,
    candidates: IncomingContentCandidates,
    mode: IncomingContentMode,
  ): Boolean {
    candidates.close()
    return false
  }
}
