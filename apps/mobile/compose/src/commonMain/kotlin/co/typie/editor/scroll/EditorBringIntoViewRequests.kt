package co.typie.editor.scroll

import androidx.compose.runtime.Composable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.remember
import kotlin.concurrent.atomics.AtomicReference
import kotlin.concurrent.atomics.ExperimentalAtomicApi

@OptIn(ExperimentalAtomicApi::class)
internal class EditorBringIntoViewRequests {
  private data class PendingBringIntoViewTarget(
    val target: EditorBringIntoViewTarget,
    val targetVersion: Long,
    val order: Long,
  )

  private data class ActiveBringIntoViewTarget(
    val target: EditorBringIntoViewTarget,
    val version: Long,
  )

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
          ActiveBringIntoViewTarget(target = pending.target, version = version),
      )
  }

  private val state = AtomicReference(State())

  fun requestForVersion(target: EditorBringIntoViewTarget, version: Long) {
    update { current ->
      current.copy(
        pendingBringIntoViewTargets =
          current.pendingBringIntoViewTargets +
            PendingBringIntoViewTarget(
              target = target,
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

  fun activateForVersion(version: Long): EditorBringIntoViewTarget? {
    while (true) {
      val current = state.load()
      current.activeBringIntoViewTarget?.let { active ->
        if (active.version == version) {
          return active.target
        }
      }

      val base = current.withoutStaleActive(version)
      val pending = base.latestEligiblePending(version)
      val next = pending?.let { base.withActive(it, version) } ?: base
      if (next === current || state.compareAndSet(current, next)) {
        return pending?.target
      }
    }
  }

  fun markApplied(version: Long, target: EditorBringIntoViewTarget): Boolean {
    while (true) {
      val current = state.load()
      val activeTarget = current.activeBringIntoViewTarget ?: return false
      if (activeTarget.version != version || activeTarget.target != target) {
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

internal val LocalEditorBringIntoViewRequests =
  compositionLocalOf<EditorBringIntoViewRequests> {
    error("No EditorBringIntoViewRequests provided")
  }

@Composable
internal fun rememberEditorBringIntoViewRequests(): EditorBringIntoViewRequests = remember {
  EditorBringIntoViewRequests()
}
