package co.typie.ui.component.sheet

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

interface SheetController<R> {
  val mode: SheetMode
  val resolvedDetents: List<ResolvedSheetDetent>
  val currentDetentId: SheetDetentId?
  val targetDetentId: SheetDetentId?
  val visibleFraction: Float
  val isVisible: Boolean
  val stackDepth: Int
  val isTopOfStack: Boolean

  fun animateTo(detentId: SheetDetentId)
  fun expand()
  fun collapse()
  fun dismiss(reason: SheetDismissReason = SheetDismissReason.Programmatic)
  fun complete(result: R)
}

internal sealed interface SheetResolutionRequest<out R> {
  data class Dismissed(
    val reason: SheetDismissReason,
  ) : SheetResolutionRequest<Nothing>

  data class Completed<R>(
    val value: R,
  ) : SheetResolutionRequest<R>
}

class SheetControllerState<R>(
  override val mode: SheetMode,
  private val dismissPolicy: SheetDismissPolicy,
) : SheetController<R> {
  private val _resolvedDetents = mutableStateListOf<ResolvedSheetDetent>()
  override val resolvedDetents: List<ResolvedSheetDetent> get() = _resolvedDetents

  override var currentDetentId: SheetDetentId? by mutableStateOf(null)
    private set

  override var targetDetentId: SheetDetentId? by mutableStateOf(null)
    private set

  override var visibleFraction: Float by mutableFloatStateOf(0f)
    private set

  override val isVisible: Boolean get() = visibleFraction > 0f

  override var stackDepth: Int by mutableIntStateOf(0)
    private set

  override var isTopOfStack: Boolean by mutableStateOf(false)
    private set

  internal var resolutionRequest: SheetResolutionRequest<R>? by mutableStateOf(null)
    private set

  internal fun updateResolvedDetents(
    detents: List<ResolvedSheetDetent>,
    initialDetentId: SheetDetentId,
    stackDepth: Int,
    isTopOfStack: Boolean,
  ) {
    _resolvedDetents.clear()
    _resolvedDetents.addAll(detents)
    if (_resolvedDetents.none { it.id == currentDetentId }) {
      currentDetentId = initialDetentId
    }
    if (_resolvedDetents.none { it.id == targetDetentId }) {
      targetDetentId = currentDetentId ?: initialDetentId
    }
    this.stackDepth = stackDepth
    this.isTopOfStack = isTopOfStack
  }

  internal fun updateVisibleFraction(value: Float) {
    visibleFraction = value.coerceIn(0f, 1f)
  }

  override fun animateTo(detentId: SheetDetentId) {
    if (_resolvedDetents.any { it.id == detentId }) {
      targetDetentId = detentId
    }
  }

  override fun expand() {
    val target = _resolvedDetents.maxByOrNull { it.height.value }?.id ?: return
    animateTo(target)
  }

  override fun collapse() {
    val target = _resolvedDetents.minByOrNull { it.height.value }?.id ?: return
    animateTo(target)
  }

  override fun dismiss(reason: SheetDismissReason) {
    if (!dismissPolicy.allows(reason) || resolutionRequest != null) {
      return
    }
    resolutionRequest = SheetResolutionRequest.Dismissed(reason)
  }

  override fun complete(result: R) {
    if (resolutionRequest != null) {
      return
    }
    resolutionRequest = SheetResolutionRequest.Completed(result)
  }

  internal fun snapToCurrentTarget() {
    currentDetentId = targetDetentId ?: currentDetentId
  }

  internal fun clearResolutionRequest() {
    resolutionRequest = null
  }
}

interface SheetScope<R> {
  val controller: SheetController<R>

  fun complete(result: R)

  fun dismiss(reason: SheetDismissReason = SheetDismissReason.Programmatic)
}

fun SheetScope<Unit>.dismiss() = dismiss(SheetDismissReason.Programmatic)
