package co.typie.editor

import kotlin.concurrent.atomics.AtomicReference
import kotlin.concurrent.atomics.ExperimentalAtomicApi
import kotlinx.collections.immutable.PersistentSet
import kotlinx.collections.immutable.toPersistentSet
import kotlinx.coroutines.CompletableDeferred

@OptIn(ExperimentalAtomicApi::class)
internal class PendingSettle(pages: Set<Int>, val requiredVersion: Long) {
  private val remaining: AtomicReference<PersistentSet<Int>> =
    AtomicReference(pages.toPersistentSet())
  private val signal: CompletableDeferred<Unit> = CompletableDeferred()

  fun markCommitted(page: Int, version: Long) {
    if (version < requiredVersion) return
    val next = updateRemaining { it.remove(page) }
    if (next.isEmpty()) signal.complete(Unit)
  }

  fun markDetached(page: Int) {
    val next = updateRemaining { it.remove(page) }
    if (next.isEmpty()) signal.complete(Unit)
  }

  fun cancel() {
    signal.cancel()
  }

  suspend fun await() {
    signal.await()
  }

  private inline fun updateRemaining(
    transform: (PersistentSet<Int>) -> PersistentSet<Int>
  ): PersistentSet<Int> {
    while (true) {
      val current = remaining.load()
      val next = transform(current)
      if (remaining.compareAndSet(current, next)) return next
    }
  }
}
