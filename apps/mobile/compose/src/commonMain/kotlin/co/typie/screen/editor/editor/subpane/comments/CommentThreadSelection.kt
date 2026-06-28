package co.typie.screen.editor.editor.subpane.comments

import co.touchlab.kermit.Logger
import co.typie.editor.ffi.StableSelection
import co.typie.graphql.fragment.CommentsSheetThread_thread
import co.typie.serialization.json
import io.sentry.kotlin.multiplatform.Sentry
import kotlinx.serialization.json.decodeFromJsonElement

internal data class CommentThreadSelections(
  val selectionsById: Map<String, StableSelection>,
  val failures: List<CommentThreadSelectionFailure>,
) {
  companion object {
    val Empty = CommentThreadSelections(selectionsById = emptyMap(), failures = emptyList())
  }
}

internal data class CommentThreadSelectionFailure(val threadId: String, val error: Exception)

internal val CommentThreadSelections.failedThreadIds: Set<String>
  get() = failures.mapTo(mutableSetOf()) { it.threadId }

internal fun decodeCommentThreadSelections(
  threads: List<CommentsSheetThread_thread>
): CommentThreadSelections {
  val selectionsById = mutableMapOf<String, StableSelection>()
  val failures = mutableListOf<CommentThreadSelectionFailure>()

  threads.forEach { thread ->
    try {
      selectionsById[thread.id] = json.decodeFromJsonElement<StableSelection>(thread.selection)
    } catch (error: Exception) {
      failures += CommentThreadSelectionFailure(threadId = thread.id, error = error)
    }
  }

  return CommentThreadSelections(selectionsById = selectionsById, failures = failures)
}

internal fun notifyCommentSelectionDecodeFailure(failure: CommentThreadSelectionFailure) {
  Logger.e(failure.error) { "Failed to decode comment thread selection: ${failure.threadId}" }
  Sentry.captureException(failure.error)
}
