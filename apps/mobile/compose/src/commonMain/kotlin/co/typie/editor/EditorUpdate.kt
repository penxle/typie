package co.typie.editor

import co.typie.editor.ffi.CommandOutcome
import co.typie.editor.ffi.EditorEvent

class EditorUpdate
internal constructor(
  val revision: Long,
  val snapshot: EditorState,
  val events: List<EditorEvent>,
  val commandOutcomes: List<CommandOutcome>,
  private val editor: Editor,
) {
  suspend fun awaitPublished(): EditorPublicationResult = editor.awaitPublished(revision)
}

sealed interface EditorPublicationResult

data class Published(val revision: Long) : EditorPublicationResult

data object NoHost : EditorPublicationResult

class EditorSurfaceUnavailableException internal constructor(val page: Int) :
  IllegalStateException("Editor surface for page $page could not publish the requested revision")
