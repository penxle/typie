package co.typie.editor.scroll

import androidx.compose.runtime.Composable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.remember
import kotlin.concurrent.atomics.AtomicReference
import kotlin.concurrent.atomics.ExperimentalAtomicApi

@OptIn(ExperimentalAtomicApi::class)
internal class EditorBringIntoViewRequests {
  data class Request(
    val target: EditorBringIntoViewTarget,
    val behavior: EditorBringIntoViewBehavior = EditorBringIntoViewBehavior.Instant,
  )

  private data class PendingBringIntoViewTarget(
    val request: Request,
    val targetVersion: Long,
    val order: Long,
  )

  private data class ActiveBringIntoViewTarget(val request: Request, val version: Long)

  private data class State(
    val pendingBringIntoViewTargets: List<PendingBringIntoViewTarget> = emptyList(),
    val activeBringIntoViewTarget: ActiveBringIntoViewTarget? = null,
    val nextRequestOrder: Long = 1L,
  ) {
    fun latestEligiblePending(version: Long): PendingBringIntoViewTarget? =
      pendingBringIntoViewTargets
        .filter { version >= it.targetVersion }
        .maxWithOrNull(
          compareBy<PendingBringIntoViewTarget> { it.targetVersion }.thenBy { it.order }
        )

    fun withoutStaleActive(version: Long): State =
      if (activeBringIntoViewTarget != null && activeBringIntoViewTarget.version != version) {
        copy(activeBringIntoViewTarget = null)
      } else {
        this
      }

    fun withActive(pending: PendingBringIntoViewTarget, version: Long): State =
      copy(
        pendingBringIntoViewTargets =
          pendingBringIntoViewTargets.filter { it.targetVersion > pending.targetVersion },
        activeBringIntoViewTarget =
          ActiveBringIntoViewTarget(request = pending.request, version = version),
      )
  }

  private val state = AtomicReference(State())

  fun requestForVersion(
    target: EditorBringIntoViewTarget,
    version: Long,
    behavior: EditorBringIntoViewBehavior = EditorBringIntoViewBehavior.Instant,
  ) {
    update { current ->
      current.copy(
        pendingBringIntoViewTargets =
          current.pendingBringIntoViewTargets +
            PendingBringIntoViewTarget(
              request = Request(target = target, behavior = behavior),
              targetVersion = version,
              order = current.nextRequestOrder,
            ),
        nextRequestOrder = current.nextRequestOrder + 1L,
      )
    }
  }

  fun cancel() {
    update { current ->
      if (
        current.pendingBringIntoViewTargets.isEmpty() && current.activeBringIntoViewTarget == null
      ) {
        current
      } else {
        State(nextRequestOrder = current.nextRequestOrder)
      }
    }
  }

  fun activateForVersion(version: Long): Request? {
    while (true) {
      val current = state.load()
      current.activeBringIntoViewTarget?.let { active ->
        if (active.version == version) {
          return active.request
        }
      }

      val base = current.withoutStaleActive(version)
      val pending = base.latestEligiblePending(version)
      val next = pending?.let { base.withActive(it, version) } ?: base
      if (next === current || state.compareAndSet(current, next)) {
        return pending?.request
      }
    }
  }

  fun markApplied(version: Long, request: Request): Boolean {
    while (true) {
      val current = state.load()
      val activeTarget = current.activeBringIntoViewTarget ?: return false
      if (activeTarget.version != version || activeTarget.request != request) {
        return false
      }
      val next = current.copy(activeBringIntoViewTarget = null)
      if (state.compareAndSet(current, next)) {
        return true
      }
    }
  }

  private inline fun update(transform: (State) -> State) {
    while (true) {
      val current = state.load()
      val next = transform(current)
      if (state.compareAndSet(current, next)) return
    }
  }
}

internal enum class EditorBringIntoViewBehavior {
  Instant,
  Smooth,
}

internal val LocalEditorBringIntoViewRequests =
  compositionLocalOf<EditorBringIntoViewRequests> {
    error("No EditorBringIntoViewRequests provided")
  }

@Composable
internal fun rememberEditorBringIntoViewRequests(): EditorBringIntoViewRequests = remember {
  EditorBringIntoViewRequests()
}
