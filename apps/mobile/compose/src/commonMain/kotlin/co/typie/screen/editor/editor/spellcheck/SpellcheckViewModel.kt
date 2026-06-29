package co.typie.screen.editor.editor.spellcheck

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.Apollo
import co.typie.graphql.LoadableMutation
import co.typie.graphql.Spellcheck_CheckSpelling_Mutation
import co.typie.graphql.executeMutation
import co.typie.graphql.type.CheckSpellingDocumentV2Input

internal data class SpellcheckResult(
  val id: String,
  val context: String,
  val corrections: List<String>,
  val explanation: String,
)

internal data class RawSpellcheckResult(
  val id: String,
  val start: Int,
  val end: Int,
  val context: String,
  val corrections: List<String>,
  val explanation: String,
)

internal class SpellcheckViewModel : ViewModel() {
  val check = LoadableMutation<Spellcheck_CheckSpelling_Mutation.Data>()

  var active by mutableStateOf(false)
    private set

  var pendingCheckText by mutableStateOf<String?>(null)
    private set

  var results by mutableStateOf<List<SpellcheckResult>>(emptyList())
    private set

  var currentCardId by mutableStateOf<String?>(null)
    private set

  var activeRangeId by mutableStateOf<String?>(null)
    private set

  var expanded by mutableStateOf(false)
    private set

  fun enterMode() {
    active = true
  }

  fun exitMode(resetLoader: Boolean = true) {
    active = false
    clear(resetLoader = resetLoader)
  }

  fun runCheck(
    documentId: String,
    text: String,
    onRawResults: (List<RawSpellcheckResult>) -> Unit,
    onError: (Throwable) -> Unit,
  ) {
    check.run(
      scope = viewModelScope,
      replaceInFlight = true,
      block = {
        Apollo.executeMutation(
          Spellcheck_CheckSpelling_Mutation(
            input = CheckSpellingDocumentV2Input(documentId = documentId, text = text)
          )
        )
      },
      onSuccess = { data ->
        onRawResults(
          data.checkSpellingDocumentV2.map { item ->
            RawSpellcheckResult(
              id = item.id,
              start = item.start,
              end = item.end,
              context = item.context,
              corrections = item.corrections,
              explanation = item.explanation,
            )
          }
        )
      },
      onError = onError,
    )
  }

  fun prepareCheck(sourceText: String) {
    clear(resetLoader = false)
    pendingCheckText = sourceText
  }

  fun isPendingCheckStale(sourceText: String, currentText: String): Boolean =
    pendingCheckText != sourceText || currentText != sourceText

  fun clearPendingCheck() {
    pendingCheckText = null
  }

  fun replaceResults(nextResults: List<SpellcheckResult>) {
    results = nextResults
    currentCardId = nextResults.firstOrNull()?.id
    activeRangeId = currentCardId
    expanded = false
  }

  fun clear(resetLoader: Boolean = true) {
    if (resetLoader) {
      check.reset()
    }
    results = emptyList()
    currentCardId = null
    activeRangeId = null
    expanded = false
    pendingCheckText = null
  }

  fun cancelCheck() {
    pendingCheckText = null
    check.cancel()
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

  fun remove(id: String, activateReplacement: Boolean): String? =
    removeIds(setOf(id), activateReplacement = activateReplacement)

  fun removeByContext(context: String, activateReplacement: Boolean): String? =
    removeIds(
      results.filter { it.context == context }.mapTo(mutableSetOf()) { it.id },
      activateReplacement = activateReplacement,
    )

  fun cleanupStale(liveTextById: Map<String, String>): Set<String> {
    val staleIds =
      results
        .filter { result -> liveTextById[result.id] != result.context }
        .mapTo(mutableSetOf()) { it.id }

    if (staleIds.isEmpty()) return emptySet()
    removeIds(staleIds, activateReplacement = false)
    return staleIds
  }

  private fun removeIds(ids: Set<String>, activateReplacement: Boolean): String? {
    if (ids.isEmpty()) return currentCardId

    val previousResults = results
    val firstRemovedIndex = previousResults.indexOfFirst { it.id in ids }
    val currentRemoved = currentCardId in ids
    val activeRemoved = activeRangeId in ids
    val nextResults = previousResults.filterNot { it.id in ids }
    val replacementId =
      if (firstRemovedIndex == -1 || nextResults.isEmpty()) {
        null
      } else {
        nextResults[firstRemovedIndex.coerceAtMost(nextResults.lastIndex)].id
      }

    results = nextResults
    if (currentRemoved || currentCardId !in nextResults.idSet()) {
      currentCardId = replacementId
    }
    if (activateReplacement) {
      activeRangeId = replacementId
      currentCardId = replacementId
    } else if (activeRemoved || activeRangeId !in nextResults.idSet()) {
      activeRangeId = null
    }
    if (nextResults.isEmpty()) {
      expanded = false
    }
    return replacementId
  }

  private fun List<SpellcheckResult>.idSet(): Set<String> = mapTo(mutableSetOf()) { it.id }
}
