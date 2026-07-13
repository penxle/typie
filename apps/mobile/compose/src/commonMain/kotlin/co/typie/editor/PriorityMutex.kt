package co.typie.editor

import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withTimeoutOrNull

// The fair inner Mutex admits waiters in FIFO order; the gate lets only one withLock
// caller queue on it at a time. withPriorityLock keeps FIFO admission (preserving the
// ordering of queued edits, e.g. an Undo ahead of a commit) and bypasses the gate only
// after the escalation window — a stall the fully-fair mutex would have turned into an
// ANR — leaving at most the in-flight critical section ahead of the blocked caller.
internal class PriorityMutex(private val escalationMillis: Long = 100) {
  private val gate = Mutex()
  private val lock = Mutex()

  suspend fun <T> withLock(block: suspend () -> T): T = gate.withLock { lock.withLock { block() } }

  suspend fun <T> withPriorityLock(block: suspend () -> T): T {
    val fifo =
      withTimeoutOrNull(escalationMillis) {
        gate.lock()
        try {
          lock.lock()
        } catch (e: Throwable) {
          gate.unlock()
          throw e
        }
      } != null
    if (!fifo) lock.lock()
    try {
      return block()
    } finally {
      lock.unlock()
      if (fifo) gate.unlock()
    }
  }
}
