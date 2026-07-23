package co.typie.editor

import kotlin.concurrent.atomics.AtomicReference
import kotlin.concurrent.atomics.ExperimentalAtomicApi
import kotlinx.collections.immutable.PersistentSet
import kotlinx.collections.immutable.toPersistentSet
import kotlinx.coroutines.CompletableDeferred

@OptIn(ExperimentalAtomicApi::class)
internal class PendingSettle(
  pages: Set<Int>,
  val requiredVersion: Long,
  val snapshot: EditorState? = null,
) {
  private val remaining: AtomicReference<PersistentSet<Int>> =
    AtomicReference(pages.toPersistentSet())
  private val signal: CompletableDeferred<Unit> = CompletableDeferred()

  // A newer snapshot from a tick that did not invalidate rendering, riding this settle
  // instead of demanding its own: such a tick has no frame of its own to wait for, and
  // requiring pages again would orphan it whenever a page already settled at this
  // version and never renders again.
  private val piggyback: AtomicReference<EditorState?> = AtomicReference(null)

  // Must be assigned before release(); read by the awaiter only after await() resumes.
  var supersededByNewerSettle: Boolean = false

  fun remainingPages(): Set<Int> = remaining.load()

  fun attachPiggyback(snapshot: EditorState) {
    while (true) {
      val current = piggyback.load()
      if (current != null && current.version >= snapshot.version) return
      if (piggyback.compareAndSet(current, snapshot)) return
    }
  }

  fun effectiveSnapshot(): EditorState? {
    val ridden = piggyback.load()
    val own = snapshot
    return when {
      ridden == null -> own
      own == null -> ridden
      ridden.version >= own.version -> ridden
      else -> own
    }
  }

  fun effectiveVersion(): Long = effectiveSnapshot()?.version ?: requiredVersion

  fun markCommitted(page: Int, version: Long): Boolean {
    if (version < requiredVersion) return false
    return updateRemaining { it.remove(page) }
  }

  fun markDetached(page: Int): Boolean = updateRemaining { it.remove(page) }

  fun release() {
    signal.complete(Unit)
  }

  fun cancel() {
    signal.cancel()
  }

  suspend fun await() {
    signal.await()
  }

  private inline fun updateRemaining(
    transform: (PersistentSet<Int>) -> PersistentSet<Int>
  ): Boolean {
    while (true) {
      val current = remaining.load()
      if (current.isEmpty()) return false
      val next = transform(current)
      if (remaining.compareAndSet(current, next)) return next.isEmpty()
    }
  }
}
