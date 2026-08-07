package co.typie.editor.scroll

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import co.typie.editor.EditorState
import kotlin.concurrent.atomics.AtomicLong
import kotlin.concurrent.atomics.AtomicReference
import kotlin.concurrent.atomics.ExperimentalAtomicApi
import kotlinx.coroutines.CompletableDeferred

@OptIn(ExperimentalAtomicApi::class)
internal class EditorBringIntoViewRequests(private val requestPresentation: () -> Unit = {}) {
  data class Request(val target: EditorBringIntoViewTarget, val policy: EditorBringIntoViewPolicy) {
    val behavior: EditorBringIntoViewBehavior =
      if (policy == EditorBringIntoViewPolicy.ResultReveal) {
        EditorBringIntoViewBehavior.Smooth
      } else {
        EditorBringIntoViewBehavior.Instant
      }
    internal val targetVersion = AtomicLong(UNBOUND_VERSION)
    internal val presentation = CompletableDeferred<Unit>()
  }

  private val pending = AtomicReference<Request?>(null)

  fun requestForState(
    state: EditorState,
    policy: EditorBringIntoViewPolicy,
    target: EditorState.() -> EditorBringIntoViewTarget?,
  ): Boolean {
    requestForVersion(
      target = state.target() ?: return false,
      version = state.version,
      policy = policy,
    )
    return true
  }

  fun requestForVersion(
    target: EditorBringIntoViewTarget,
    version: Long,
    policy: EditorBringIntoViewPolicy,
  ): Request = declare(target = target, policy = policy).also { bind(it, version) }

  fun declare(target: EditorBringIntoViewTarget, policy: EditorBringIntoViewPolicy): Request {
    val request = Request(target = target, policy = policy)
    pending.exchange(request)?.presentation?.complete(Unit)
    requestPresentation()
    return request
  }

  fun bind(request: Request, version: Long): Boolean {
    if (pending.load() !== request) return false
    val current = request.targetVersion.load()
    if (current != UNBOUND_VERSION) return current == version
    val bound =
      request.targetVersion.compareAndSet(UNBOUND_VERSION, version) && pending.load() === request
    if (bound) requestPresentation()
    return bound
  }

  fun cancel() {
    pending.exchange(null)?.let { request ->
      request.presentation.complete(Unit)
      requestPresentation()
    }
  }

  fun discard(request: Request) {
    if (pending.compareAndSet(request, null)) {
      request.presentation.complete(Unit)
      requestPresentation()
    }
  }

  fun activateForVersion(version: Long): Request? {
    val request = pending.load() ?: return null
    val targetVersion =
      request.targetVersion.load().takeUnless { it == UNBOUND_VERSION } ?: return null
    val eligible =
      if (
        request.target is EditorBringIntoViewTarget.PageRects ||
          request.policy == EditorBringIntoViewPolicy.PointerCursorGuard
      ) {
        version == targetVersion
      } else {
        version >= targetVersion
      }
    return request.takeIf { eligible }
  }

  fun discardObsoleteForVersion(version: Long) {
    val request = pending.load() ?: return
    val targetVersion = request.targetVersion.load().takeUnless { it == UNBOUND_VERSION } ?: return
    if (
      (request.target is EditorBringIntoViewTarget.PageRects ||
        request.policy == EditorBringIntoViewPolicy.PointerCursorGuard) && version > targetVersion
    ) {
      discard(request)
    }
  }

  fun discardFailedForVersion(version: Long) {
    activateForVersion(version)?.let(::discard)
  }

  fun markPresented(version: Long, request: Request): Boolean {
    if (activateForVersion(version) !== request || !pending.compareAndSet(request, null)) {
      return false
    }
    request.presentation.complete(Unit)
    requestPresentation()
    return true
  }

  suspend fun awaitPresentation(request: Request) {
    request.presentation.await()
  }

  private companion object {
    const val UNBOUND_VERSION = Long.MIN_VALUE
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
internal fun rememberEditorBringIntoViewRequests(
  requestPresentation: () -> Unit = {}
): EditorBringIntoViewRequests {
  val currentRequestPresentation = rememberUpdatedState(requestPresentation)
  val requests = remember { EditorBringIntoViewRequests { currentRequestPresentation.value() } }
  DisposableEffect(requests) { onDispose { requests.cancel() } }
  return requests
}
