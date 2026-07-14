package co.typie.screen.editor.editor.spellcheck

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.contract.LoadableState
import co.typie.graphql.Apollo
import co.typie.graphql.Spellcheck_CheckSpelling_Mutation
import co.typie.graphql.executeMutation
import co.typie.graphql.type.CheckSpellingDocumentV2Input
import kotlin.concurrent.atomics.AtomicBoolean
import kotlin.concurrent.atomics.AtomicReference
import kotlin.concurrent.atomics.ExperimentalAtomicApi
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch

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

internal data class PendingSpellcheck(val sourceText: String, val run: SpellcheckRun)

@OptIn(ExperimentalAtomicApi::class)
internal class SpellcheckRun {
  private val valid = AtomicBoolean(true)

  fun isValid(): Boolean = valid.load()

  fun invalidate(): Boolean = valid.compareAndSet(expectedValue = true, newValue = false)
}

@OptIn(ExperimentalAtomicApi::class)
internal class SpellcheckViewModel(
  private val request: suspend (documentId: String, text: String) -> List<RawSpellcheckResult> =
    ::requestSpellcheck
) : ViewModel() {
  private var checkJob: Job? = null
  private val currentRun = AtomicReference<SpellcheckRun?>(null)

  var readiness: LoadableState<Unit> by mutableStateOf(LoadableState.Idle)
    private set

  val loading: Boolean
    get() = readiness is LoadableState.Loading

  val ready: Boolean
    get() = readiness is LoadableState.Success

  val error: Throwable?
    get() = (readiness as? LoadableState.Error)?.exception

  var active by mutableStateOf(false)
    private set

  var pendingCheck by mutableStateOf<PendingSpellcheck?>(null)
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

  fun exitMode() {
    active = false
    resetCheck()
  }

  fun runCheck(
    documentId: String,
    sourceText: suspend () -> String,
    beforeRequest: suspend (SpellcheckRun, String) -> Unit,
    prepareResults:
      suspend (List<RawSpellcheckResult>, SpellcheckRun, String) -> List<SpellcheckResult>,
    onReady: (List<SpellcheckResult>) -> Unit,
    onError: suspend (Throwable, SpellcheckRun) -> Unit,
  ) {
    if (loading) return

    clearResults()
    pendingCheck = null
    val run = SpellcheckRun()
    currentRun.exchange(run)?.invalidate()
    readiness = LoadableState.Loading
    val nextJob =
      viewModelScope.launch(start = CoroutineStart.LAZY) {
        try {
          val text = sourceText()
          ensureCurrent(run)
          pendingCheck = PendingSpellcheck(sourceText = text, run = run)
          beforeRequest(run, text)
          ensureCurrent(run)
          val rawResults = request(documentId, text)
          ensureCurrent(run)
          val prepared = prepareResults(rawResults, run, text)
          ensureCurrent(run)
          replaceResults(prepared)
          pendingCheck = null
          onReady(prepared)
          ensureCurrent(run)
          readiness = LoadableState.Success(Unit)
        } catch (error: CancellationException) {
          throw error
        } catch (error: Throwable) {
          if (!beginCleanup(run)) return@launch
          try {
            clearResults()
            if (pendingCheck?.run === run) pendingCheck = null
            val finalError =
              try {
                onError(error, run)
                error
              } catch (cleanupCancellation: CancellationException) {
                throw cleanupCancellation
              } catch (cleanupError: Throwable) {
                cleanupError
              }
            if (ownsCleanup(run) && loading) {
              readiness = LoadableState.Error(finalError)
            }
          } finally {
            finishCleanup(run)
          }
        } finally {
          if (checkJob === coroutineContext[Job]) checkJob = null
        }
      }
    checkJob = nextJob
    nextJob.start()
  }

  fun isCurrent(run: SpellcheckRun): Boolean = currentRun.load() === run && run.isValid()

  fun ownsCleanup(run: SpellcheckRun): Boolean = currentRun.load() === run && !run.isValid()

  fun hasNoActiveRun(): Boolean = currentRun.load()?.isValid() != true

  fun cancelCheck(run: SpellcheckRun): Boolean {
    if (!beginCleanup(run)) return false
    checkJob?.cancel()
    checkJob = null
    if (pendingCheck?.run === run) pendingCheck = null
    readiness = LoadableState.Idle
    return true
  }

  fun finishCleanup(run: SpellcheckRun) {
    if (ownsCleanup(run)) currentRun.compareAndSet(expectedValue = run, newValue = null)
  }

  private fun ensureCurrent(run: SpellcheckRun) {
    if (!isCurrent(run)) throw CancellationException("Spellcheck run superseded")
  }

  private fun beginCleanup(run: SpellcheckRun): Boolean {
    if (currentRun.load() !== run) return false
    return run.invalidate()
  }

  private fun resetCheck() {
    currentRun.exchange(null)?.invalidate()
    checkJob?.cancel()
    checkJob = null
    readiness = LoadableState.Idle
    clearResults()
    pendingCheck = null
  }

  fun replaceResults(nextResults: List<SpellcheckResult>) {
    results = nextResults
    currentCardId = nextResults.firstOrNull()?.id
    activeRangeId = currentCardId
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

  private fun clearResults() {
    results = emptyList()
    currentCardId = null
    activeRangeId = null
    expanded = false
  }
}

private suspend fun requestSpellcheck(documentId: String, text: String): List<RawSpellcheckResult> =
  Apollo.executeMutation(
      Spellcheck_CheckSpelling_Mutation(
        input = CheckSpellingDocumentV2Input(documentId = documentId, text = text)
      )
    )
    .checkSpellingDocumentV2
    .map { item ->
      RawSpellcheckResult(
        id = item.id,
        start = item.start,
        end = item.end,
        context = item.context,
        corrections = item.corrections,
        explanation = item.explanation,
      )
    }
