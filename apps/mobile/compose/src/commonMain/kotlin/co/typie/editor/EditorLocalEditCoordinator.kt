package co.typie.editor

import kotlin.coroutines.AbstractCoroutineContextElement
import kotlin.coroutines.CoroutineContext
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.withContext

internal class LocalEditQuiescence
internal constructor(
  private val awaitCompletion: suspend () -> Result<Unit>,
  private val resumeEditing: () -> Unit,
) {
  suspend fun await(): Result<Unit> = awaitCompletion()

  fun resume() {
    resumeEditing()
  }
}

internal class LocalEdit
internal constructor(private val coordinator: EditorLocalEditCoordinator, internal val id: Long) :
  AbstractCoroutineContextElement(Key) {
  companion object Key : CoroutineContext.Key<LocalEdit>

  internal fun belongsTo(coordinator: EditorLocalEditCoordinator): Boolean =
    this.coordinator === coordinator

  internal fun complete() {
    coordinator.complete(this)
  }

  internal fun fail(cause: Throwable) {
    coordinator.fail(this, cause)
  }
}

internal class EditorLocalEditCoordinator {
  private data class State(
    val nextId: Long = 1L,
    val accepting: Boolean = true,
    val pending: Set<Long> = emptySet(),
    val failures: Map<Long, Throwable> = emptyMap(),
  )

  private val state = MutableStateFlow(State())

  fun register(): LocalEdit? {
    while (true) {
      val current = state.value
      if (!current.accepting) return null
      val localEdit = LocalEdit(this, current.nextId)
      val next =
        current.copy(nextId = current.nextId + 1L, pending = current.pending + localEdit.id)
      if (state.compareAndSet(current, next)) return localEdit
    }
  }

  suspend fun <T> run(block: suspend () -> T): T {
    val current = currentCoroutineContext()[LocalEdit]
    if (current?.belongsTo(this) == true) return block()

    val localEdit = register() ?: throw CancellationException("Editor local edits quiesced")
    try {
      return withContext(localEdit) { block() }.also { localEdit.complete() }
    } catch (e: Throwable) {
      localEdit.fail(e)
      throw e
    }
  }

  fun quiesce(): LocalEditQuiescence {
    val through = closeAdmission()
    return LocalEditQuiescence(
      awaitCompletion = { awaitThrough(through) },
      resumeEditing = { resumeThrough(through) },
    )
  }

  internal fun complete(localEdit: LocalEdit) {
    update(localEdit) { current ->
      current.copy(
        pending = current.pending - localEdit.id,
        failures = current.failures - localEdit.id,
      )
    }
  }

  internal fun fail(localEdit: LocalEdit, cause: Throwable) {
    update(localEdit) { current ->
      current.copy(
        pending = current.pending - localEdit.id,
        failures = current.failures + (localEdit.id to cause),
      )
    }
  }

  private fun closeAdmission(): Long {
    while (true) {
      val current = state.value
      val through = current.nextId - 1L
      if (!current.accepting) return through
      if (state.compareAndSet(current, current.copy(accepting = false))) return through
    }
  }

  private suspend fun awaitThrough(through: Long): Result<Unit> {
    val settled = state.first { current -> current.pending.none { it <= through } }
    val failure =
      settled.failures.entries.filter { it.key <= through }.minByOrNull { it.key }?.value
    return if (failure == null) Result.success(Unit) else Result.failure(failure)
  }

  private fun resumeThrough(through: Long) {
    while (true) {
      val current = state.value
      val next =
        current.copy(accepting = true, failures = current.failures.filterKeys { it > through })
      if (state.compareAndSet(current, next)) return
    }
  }

  private inline fun update(localEdit: LocalEdit, transform: (State) -> State) {
    while (true) {
      val current = state.value
      if (localEdit.id !in current.pending) return
      if (state.compareAndSet(current, transform(current))) return
    }
  }
}
