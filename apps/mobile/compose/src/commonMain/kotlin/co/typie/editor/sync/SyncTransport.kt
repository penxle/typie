package co.typie.editor.sync

import co.typie.editor.sync.ws.SyncWsException
import kotlinx.coroutines.flow.Flow

data class PullResult(
  val changesets: List<ByteArray>,
  val seq: String,
  val heads: ByteArray,
  val durableHeads: ByteArray,
  val needsReload: Boolean,
)

data class RemoteChangesetEvent(
  val changesets: List<ByteArray>,
  val seq: String,
  val heads: ByteArray,
  val durableHeads: ByteArray,
)

interface SyncTransport {
  suspend fun push(changesets: ByteArray): PushResult

  suspend fun pull(sinceSeq: String?): PullResult

  fun subscribe(sinceSeq: String?): Flow<RemoteChangesetEvent>
}

fun isPermanentSyncError(error: Throwable): Boolean = isPermanentSyncError(error, mutableSetOf())

private fun isPermanentSyncError(error: Throwable, seen: MutableSet<Throwable>): Boolean {
  var current: Throwable? = error
  while (current != null && seen.add(current)) {
    if (current is SyncWsException) return current.permanent
    for (suppressed in current.suppressedExceptions) {
      if (isPermanentSyncError(suppressed, seen)) return true
    }
    current = current.cause
  }
  return false
}

fun isSubscriptionRequiredSyncError(error: Throwable): Boolean =
  isSubscriptionRequiredSyncError(error, mutableSetOf())

private fun isSubscriptionRequiredSyncError(
  error: Throwable,
  seen: MutableSet<Throwable>,
): Boolean {
  var current: Throwable? = error
  while (current != null && seen.add(current)) {
    if (current is SyncWsException && current.code == "subscription_required") return true
    for (suppressed in current.suppressedExceptions) {
      if (isSubscriptionRequiredSyncError(suppressed, seen)) return true
    }
    current = current.cause
  }
  return false
}
