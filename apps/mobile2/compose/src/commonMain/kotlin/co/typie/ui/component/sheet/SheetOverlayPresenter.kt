package co.typie.ui.component.sheet

import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.ui.unit.dp
import kotlin.coroutines.resume
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.suspendCancellableCoroutine

internal interface SheetOverlayPresenter {
  suspend fun <R> present(
    spec: SheetOverlaySpec = SheetOverlaySpec(),
    content: @Composable SheetScope<R>.() -> Unit,
  ): SheetResult<R>
}

internal class SheetOverlayEntry<R>
internal constructor(
  val spec: SheetOverlaySpec,
  val controller: SheetControllerState<R>,
  val content: @Composable SheetScope<R>.() -> Unit,
  private val onResolved: (SheetResult<R>) -> Unit,
) {
  val mode: SheetMode
    get() = spec.mode

  val isTopOfStack: Boolean
    get() = controller.isTopOfStack

  internal fun resolve(request: SheetResolutionRequest<R>) {
    when (request) {
      is SheetResolutionRequest.Completed -> onResolved(SheetResult.Completed(request.value))
      is SheetResolutionRequest.Dismissed -> onResolved(SheetResult.Dismissed(request.reason))
    }
  }
}

internal class SheetOverlayPresenterState : SheetOverlayPresenter {
  private val _entries = mutableStateListOf<SheetOverlayEntry<*>>()
  internal val entries: List<SheetOverlayEntry<*>>
    get() = _entries

  override suspend fun <R> present(
    spec: SheetOverlaySpec,
    content: @Composable SheetScope<R>.() -> Unit,
  ): SheetResult<R> {
    var entry: SheetOverlayEntry<R>? = null
    try {
      return suspendCancellableCoroutine { continuation ->
        val controller =
          SheetControllerState<R>(mode = spec.mode, dismissPolicy = spec.dismissPolicy)
        val overlayEntry =
          SheetOverlayEntry(
            spec = spec,
            controller = controller,
            content = content,
            onResolved = { result ->
              if (continuation.isActive) {
                continuation.resume(result)
              }
            },
          )
        continuation.invokeOnCancellation {
          if (it is CancellationException) {
            removeEntry(overlayEntry)
          }
        }
        entry = overlayEntry
        _entries.add(overlayEntry)
        recalculateStackMetadata()
      }
    } finally {
      entry?.let { removeEntry(it) }
    }
  }

  private fun removeEntry(entry: SheetOverlayEntry<*>) {
    if (_entries.remove(entry)) {
      recalculateStackMetadata()
    }
  }

  private fun recalculateStackMetadata() {
    val lastIndex = _entries.lastIndex
    _entries.forEachIndexed { index, entry ->
      entry.controller.updateResolvedDetents(
        detents =
          entry.controller.resolvedDetents.ifEmpty {
            listOf(ResolvedSheetDetent(entry.spec.sizePolicy.initialDetentId(), 0.dp))
          },
        initialDetentId = entry.spec.sizePolicy.initialDetentId(),
        stackDepth = index,
        isTopOfStack = index == lastIndex,
      )
    }
  }
}
