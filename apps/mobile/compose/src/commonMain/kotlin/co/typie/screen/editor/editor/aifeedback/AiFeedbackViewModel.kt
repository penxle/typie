package co.typie.screen.editor.editor.aifeedback

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel

internal data class AiFeedbackResult(
  val id: String,
  val startText: String,
  val endText: String,
  val feedback: String,
  val category: String?,
)

internal data class RawAiFeedbackResult(
  val id: String,
  val start: Int,
  val end: Int,
  val startText: String,
  val endText: String,
  val feedback: String,
  val category: String?,
)

internal data class AiFeedbackProgress(val current: Int, val total: Int, val phase: String)

internal class AiFeedbackViewModel : ViewModel() {
  private var currentAnalysisRunId = 0L

  var active by mutableStateOf(false)
    private set

  var loading by mutableStateOf(false)
    private set

  var hasCompleted by mutableStateOf(false)
    private set

  var pendingAnalysisText by mutableStateOf<String?>(null)
    private set

  var progress by mutableStateOf<AiFeedbackProgress?>(null)
    private set

  var results by mutableStateOf<List<AiFeedbackResult>>(emptyList())
    private set

  var currentCardId by mutableStateOf<String?>(null)
    private set

  var activeRangeId by mutableStateOf<String?>(null)
    private set

  var expanded by mutableStateOf(false)
    private set

  val resultCount: Int
    get() = results.size

  fun enterMode() {
    active = true
  }

  fun exitMode() {
    active = false
    clear()
  }

  fun prepareAnalysis(sourceText: String): Long {
    val runId = nextAnalysisRunId()
    clearState()
    loading = true
    pendingAnalysisText = sourceText
    return runId
  }

  fun isCurrentAnalysisRun(runId: Long): Boolean = runId == currentAnalysisRunId

  fun isPendingAnalysisStale(sourceText: String, currentText: String): Boolean =
    pendingAnalysisText != sourceText || currentText != sourceText

  fun appendResult(result: AiFeedbackResult) {
    if (results.any { it.id == result.id }) return
    val wasEmpty = results.isEmpty()
    results = results + result
    if (wasEmpty) {
      currentCardId = result.id
      activeRangeId = result.id
      expanded = false
    }
  }

  fun updateProgress(nextProgress: AiFeedbackProgress?) {
    progress = nextProgress
  }

  fun complete() {
    loading = false
    hasCompleted = true
    pendingAnalysisText = null
    progress = null
  }

  fun fail() {
    loading = false
    pendingAnalysisText = null
    progress = null
  }

  fun cancelAnalysis() {
    clear()
  }

  fun clear() {
    nextAnalysisRunId()
    clearState()
  }

  private fun clearState() {
    loading = false
    hasCompleted = false
    pendingAnalysisText = null
    progress = null
    results = emptyList()
    currentCardId = null
    activeRangeId = null
    expanded = false
  }

  fun updateExpanded(nextExpanded: Boolean) {
    expanded = nextExpanded
  }

  fun activate(id: String?) {
    if (id == null || results.none { it.id == id }) {
      activeRangeId = null
      return
    }
    currentCardId = id
    activeRangeId = id
  }

  fun setCurrent(id: String?) {
    currentCardId = id?.takeIf { nextId -> results.any { it.id == nextId } }
  }

  fun remove(id: String, activateReplacement: Boolean): String? {
    return removeIds(setOf(id), activateReplacement = activateReplacement)
  }

  fun cleanupMissingRanges(liveIds: Set<String>): Set<String> {
    val missingIds = results.filter { it.id !in liveIds }.mapTo(mutableSetOf()) { it.id }
    if (missingIds.isEmpty()) return emptySet()
    removeIds(missingIds, activateReplacement = false)
    return missingIds
  }

  private fun removeIds(ids: Set<String>, activateReplacement: Boolean): String? {
    if (ids.isEmpty()) return currentCardId

    val previousResults = results
    val removedIndex = previousResults.indexOfFirst { it.id in ids }
    if (removedIndex == -1) return currentCardId

    val nextResults = previousResults.filterNot { it.id in ids }
    val replacementId =
      if (nextResults.isEmpty()) {
        null
      } else {
        nextResults[removedIndex.coerceAtMost(nextResults.lastIndex)].id
      }
    val currentRemoved = currentCardId in ids
    val activeRemoved = activeRangeId in ids

    results = nextResults
    if (currentRemoved || currentCardId !in nextResults.idSet()) {
      currentCardId = replacementId
    }
    if (activateReplacement) {
      currentCardId = replacementId
      activeRangeId = replacementId
    } else if (activeRemoved || activeRangeId !in nextResults.idSet()) {
      activeRangeId = null
    }
    if (nextResults.isEmpty()) {
      expanded = false
    }
    return replacementId
  }

  private fun nextAnalysisRunId(): Long {
    currentAnalysisRunId += 1
    return currentAnalysisRunId
  }

  private fun List<AiFeedbackResult>.idSet(): Set<String> = mapTo(mutableSetOf()) { it.id }
}
