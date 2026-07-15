package co.typie.editor.interaction

import androidx.compose.foundation.gestures.FlingBehavior
import androidx.compose.foundation.gestures.Scrollable2DState
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.nestedscroll.NestedScrollDispatcher
import androidx.compose.ui.input.pointer.PointerEvent
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.input.pointer.PointerInputChange
import androidx.compose.ui.input.pointer.PointerType
import androidx.compose.ui.input.pointer.changedToUpIgnoreConsumed
import androidx.compose.ui.input.pointer.isAltPressed
import androidx.compose.ui.input.pointer.isCtrlPressed
import androidx.compose.ui.input.pointer.isMetaPressed
import androidx.compose.ui.input.pointer.isShiftPressed
import androidx.compose.ui.node.ModifierNodeElement
import androidx.compose.ui.node.PointerInputModifierNode
import androidx.compose.ui.node.SemanticsModifierNode
import androidx.compose.ui.node.requireLayoutCoordinates
import androidx.compose.ui.semantics.SemanticsPropertyReceiver
import androidx.compose.ui.semantics.scrollBy
import androidx.compose.ui.semantics.scrollByOffset
import androidx.compose.ui.unit.IntSize
import co.typie.editor.ffi.InputModifiers
import co.typie.editor.viewport.normalizeEditorViewportWheelZoomDelta
import kotlin.math.abs
import kotlin.math.min
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

private const val EditorTapSlopDp = 8f
private const val WheelBurstGapMs = 56L
private const val WheelTailDeltaPx = 0.8f
private const val WheelTailStreakToReset = 3
private const val WheelModeSwitchMinDeltaPx = 1.5f

internal fun Modifier.editorInteractions(
  interactionController: EditorInteractionController,
  geometry: EditorInteractionGeometry,
  screenPointerSequence: EditorScreenPointerSequence,
  scrollableState: Scrollable2DState? = null,
  nestedScrollDispatcher: NestedScrollDispatcher? = null,
  flingBehavior: FlingBehavior? = null,
  touchSlop: Float = 0f,
  maximumFlingVelocity: Float = Float.MAX_VALUE,
  density: Float,
  enabled: Boolean = true,
  onViewportWheelScroll: () -> Unit = {},
  onNestedScrollCancel: () -> Unit = {},
): Modifier =
  this then
    EditorInteractionsElement(
      interactionController = interactionController,
      geometry = geometry,
      screenPointerSequence = screenPointerSequence,
      scrollableState = scrollableState,
      nestedScrollDispatcher = nestedScrollDispatcher,
      flingBehavior = flingBehavior,
      touchSlop = touchSlop,
      maximumFlingVelocity = maximumFlingVelocity,
      density = density,
      enabled = enabled,
      onViewportWheelScroll = onViewportWheelScroll,
      onNestedScrollCancel = onNestedScrollCancel,
    )

private data class EditorInteractionsElement(
  private val interactionController: EditorInteractionController,
  private val geometry: EditorInteractionGeometry,
  private val screenPointerSequence: EditorScreenPointerSequence,
  private val scrollableState: Scrollable2DState?,
  private val nestedScrollDispatcher: NestedScrollDispatcher?,
  private val flingBehavior: FlingBehavior?,
  private val touchSlop: Float,
  private val maximumFlingVelocity: Float,
  private val density: Float,
  private val enabled: Boolean,
  private val onViewportWheelScroll: () -> Unit,
  private val onNestedScrollCancel: () -> Unit,
) : ModifierNodeElement<EditorInteractionsNode>() {
  override fun create(): EditorInteractionsNode =
    EditorInteractionsNode(
      interactionController = interactionController,
      geometry = geometry,
      screenPointerSequence = screenPointerSequence,
      scrollableState = scrollableState,
      nestedScrollDispatcher = nestedScrollDispatcher,
      flingBehavior = flingBehavior,
      touchSlop = touchSlop,
      maximumFlingVelocity = maximumFlingVelocity,
      density = density,
      enabled = enabled,
      onViewportWheelScroll = onViewportWheelScroll,
      onNestedScrollCancel = onNestedScrollCancel,
    )

  override fun update(node: EditorInteractionsNode) {
    node.update(
      interactionController = interactionController,
      geometry = geometry,
      screenPointerSequence = screenPointerSequence,
      scrollableState = scrollableState,
      nestedScrollDispatcher = nestedScrollDispatcher,
      flingBehavior = flingBehavior,
      touchSlop = touchSlop,
      maximumFlingVelocity = maximumFlingVelocity,
      density = density,
      enabled = enabled,
      onViewportWheelScroll = onViewportWheelScroll,
      onNestedScrollCancel = onNestedScrollCancel,
    )
  }
}

private class EditorInteractionsNode(
  var interactionController: EditorInteractionController,
  var geometry: EditorInteractionGeometry,
  var screenPointerSequence: EditorScreenPointerSequence,
  var scrollableState: Scrollable2DState?,
  var nestedScrollDispatcher: NestedScrollDispatcher?,
  var flingBehavior: FlingBehavior?,
  var touchSlop: Float,
  var maximumFlingVelocity: Float,
  var density: Float,
  var enabled: Boolean,
  var onViewportWheelScroll: () -> Unit,
  var onNestedScrollCancel: () -> Unit,
) : Modifier.Node(), PointerInputModifierNode, SemanticsModifierNode, EditorScreenPointerListener {
  private val pointers = mutableMapOf<Long, PointerType>()
  private val singlePointerStreams = mutableSetOf<Long>()
  private var suppressUntilAllUp = false
  private var wheelLastEventMillis: Long? = null
  private var wheelLowDeltaStreak = 0
  private var wheelZoomActive = false
  private var wheelZoomTimeoutJob: Job? = null
  private val scrollDriver =
    EditorViewportScrollDriver(
      scrollableState = { scrollableState },
      nestedScrollDispatcher = { nestedScrollDispatcher },
      flingBehavior = { flingBehavior },
      touchSlopProvider = { touchSlop },
      maximumFlingVelocityProvider = { maximumFlingVelocity },
      launch = { block -> coroutineScope.launch { block() } },
      onCancel = { onNestedScrollCancel() },
    )

  override fun onAttach() {
    screenPointerSequence.attach(this)
  }

  fun update(
    interactionController: EditorInteractionController,
    geometry: EditorInteractionGeometry,
    screenPointerSequence: EditorScreenPointerSequence,
    scrollableState: Scrollable2DState?,
    nestedScrollDispatcher: NestedScrollDispatcher?,
    flingBehavior: FlingBehavior?,
    touchSlop: Float,
    maximumFlingVelocity: Float,
    density: Float,
    enabled: Boolean,
    onViewportWheelScroll: () -> Unit,
    onNestedScrollCancel: () -> Unit,
  ) {
    val inputOwnerChanged =
      this.interactionController !== interactionController ||
        this.geometry !== geometry ||
        this.screenPointerSequence !== screenPointerSequence ||
        this.scrollableState !== scrollableState ||
        this.nestedScrollDispatcher !== nestedScrollDispatcher ||
        this.flingBehavior !== flingBehavior
    if (inputOwnerChanged || !enabled) {
      cancelInteraction(clearSuppression = true)
    }
    if (this.screenPointerSequence !== screenPointerSequence) {
      this.screenPointerSequence.detach(this)
      screenPointerSequence.attach(this)
    }
    this.interactionController = interactionController
    this.geometry = geometry
    this.screenPointerSequence = screenPointerSequence
    this.scrollableState = scrollableState
    this.nestedScrollDispatcher = nestedScrollDispatcher
    this.flingBehavior = flingBehavior
    this.touchSlop = touchSlop
    this.maximumFlingVelocity = maximumFlingVelocity
    this.density = density
    this.enabled = enabled
    this.onViewportWheelScroll = onViewportWheelScroll
    this.onNestedScrollCancel = onNestedScrollCancel
  }

  override fun onPointerEvent(pointerEvent: PointerEvent, pass: PointerEventPass, bounds: IntSize) {
    if (pass != PointerEventPass.Main) {
      return
    }
    if (pointerEvent.type == PointerEventType.Scroll) {
      handlePointerSignal(pointerEvent)
      return
    }
    finishWheelZoom()
    if (!enabled || density <= 0f) {
      cancelInteraction(clearSuppression = true)
      return
    }

    interactionController.updateTapSlop(tapSlopPx = EditorTapSlopDp * density)
    interactionController.updateColumnResizeSlop(
      dragSlopPx = min(touchSlop, EditorTapSlopDp * density)
    )
    registerPointerDowns(pointerEvent)
    val pressedTouchChanges = pressedTouchChanges(pointerEvent)

    if (screenPointerSequence.hasForeignTouchPointer(::isEditorTouchPointer)) {
      suppressMixedSequence()
    }
    if (suppressUntilAllUp) {
      consumeEditorChanges(pointerEvent)
      finishReleasedPointers(pointerEvent)
      return
    }
    if (pointers.values.count { pointerType -> pointerType == PointerType.Touch } > 2) {
      cancelAndSuppress(pointerEvent)
      return
    }
    if (pressedTouchChanges.size == 2) {
      val sample = resolvePinchSample(pressedTouchChanges)
      if (sample == null || !interactionController.onPinchSample(sample)) {
        cancelAndSuppress(pointerEvent)
        return
      }
      consumeEditorChanges(pointerEvent)
      finishReleasedPointers(pointerEvent)
      return
    }
    if (pressedTouchChanges.size == 1) {
      val survivor = pressedTouchChanges.single()
      val rootPosition = positionInRoot(survivor.position)
      if (
        interactionController.endPinchAndResumeViewportPan(
          pointerId = survivor.id.value,
          position = rootPosition,
          nowMillis = survivor.uptimeMillis,
          driver = scrollDriver,
        )
      ) {
        singlePointerStreams += survivor.id.value
        consumeEditorChanges(pointerEvent)
        finishReleasedPointers(pointerEvent)
        return
      }
    } else if (interactionController.onPinchEnd()) {
      consumeEditorChanges(pointerEvent)
      finishReleasedPointers(pointerEvent)
      return
    }

    forwardSinglePointerChanges(pointerEvent)
    finishReleasedPointers(pointerEvent)
  }

  override fun onCancelPointerInput() {
    cancelInteraction(clearSuppression = true)
  }

  override fun onScreenPointerMembershipChanged() {
    if (
      pointers.values.none { pointerType -> pointerType == PointerType.Touch } ||
        !screenPointerSequence.hasForeignTouchPointer(::isEditorTouchPointer)
    ) {
      return
    }
    suppressMixedSequence()
  }

  override fun onGlobalAllUp() {
    pointers.clear()
    singlePointerStreams.clear()
    suppressUntilAllUp = false
  }

  override fun SemanticsPropertyReceiver.applySemantics() {
    if (!enabled || !scrollDriver.isAvailable) {
      return
    }
    scrollBy { x, y ->
      scrollDriver.launchSemanticsScroll(Offset(x, y))
      true
    }
    scrollByOffset(scrollDriver::performSemanticsScroll)
  }

  private fun registerPointerDowns(pointerEvent: PointerEvent) {
    pointerEvent.changes
      .filter { change -> change.pressed && !change.previousPressed }
      .forEach { change -> pointers[change.id.value] = change.type }
  }

  private fun isEditorTouchPointer(pointerId: Long): Boolean =
    pointers[pointerId] == PointerType.Touch

  private fun pressedTouchChanges(pointerEvent: PointerEvent): List<PointerInputChange> =
    pointerEvent.changes.filter { change ->
      change.pressed && pointers[change.id.value] == PointerType.Touch
    }

  private fun forwardSinglePointerChanges(pointerEvent: PointerEvent) {
    pointerEvent.changes
      .filter { change -> change.pressed && !change.previousPressed }
      .forEach { change ->
        val rootPosition = positionInRoot(change.position)
        val tapEnabled = geometry.isTapEligible(change.position)
        val editorPosition = geometry.resolveInteractionPosition(change.position)
        singlePointerStreams += change.id.value
        if (
          interactionController.onPointerDown(
            pointerId = change.id.value,
            position = editorPosition,
            nowMillis = change.uptimeMillis,
            tapEnabled = tapEnabled,
            inputModifiers = pointerEvent.inputModifiers(),
            positionInRoot = rootPosition,
            touchPanDriver = if (change.type == PointerType.Touch) scrollDriver else null,
          )
        ) {
          change.consume()
        }
      }

    pointerEvent.changes
      .filter { change ->
        change.pressed && change.previousPressed && change.id.value in singlePointerStreams
      }
      .forEach { change ->
        val rootPosition = positionInRoot(change.position)
        val editorPosition = geometry.resolveInteractionPosition(change.position)
        if (
          interactionController.onPointerMove(
            pointerId = change.id.value,
            position = editorPosition,
            positionInRoot = rootPosition,
            nowMillis = change.uptimeMillis,
            consumed = change.isConsumed,
          )
        ) {
          change.consume()
        }
      }

    pointerEvent.changes
      .filter { change ->
        change.changedToUpIgnoreConsumed() && change.id.value in singlePointerStreams
      }
      .forEach { change ->
        val rootPosition = positionInRoot(change.position)
        val editorPosition = geometry.resolveInteractionPosition(change.position)
        if (
          interactionController.onPointerUp(
            pointerId = change.id.value,
            position = editorPosition,
            positionInRoot = rootPosition,
            nowMillis = change.uptimeMillis,
          )
        ) {
          change.consume()
        }
        singlePointerStreams -= change.id.value
      }
  }

  private fun resolvePinchSample(changes: List<PointerInputChange>): EditorPinchSample? =
    resolveEditorPinchSample(
      positionsInRoot = changes.map { change -> positionInRoot(change.position) }
    )

  private fun positionInRoot(position: Offset): Offset =
    requireLayoutCoordinates().localToRoot(position)

  private fun finishReleasedPointers(pointerEvent: PointerEvent) {
    pointerEvent.changes
      .filter { change -> !change.pressed }
      .forEach { change ->
        pointers.remove(change.id.value)
        singlePointerStreams -= change.id.value
      }
    if (pointers.isEmpty() && !screenPointerSequence.hasScreenPointers) {
      suppressUntilAllUp = false
    }
  }

  private fun cancelAndSuppress(pointerEvent: PointerEvent) {
    interactionController.cancel()
    scrollDriver.cancel()
    suppressUntilAllUp = pointerEvent.changes.any { change -> change.pressed }
    consumeEditorChanges(pointerEvent)
    finishReleasedPointers(pointerEvent)
  }

  private fun suppressMixedSequence() {
    val hadEditorInteraction =
      singlePointerStreams.isNotEmpty() ||
        interactionController.interactionMode != EditorInteractionMode.Idle
    suppressUntilAllUp = true
    if (hadEditorInteraction) {
      interactionController.cancel()
    }
    scrollDriver.cancel()
    finishWheelZoom()
  }

  private fun consumeEditorChanges(pointerEvent: PointerEvent) {
    pointerEvent.changes
      .filter { change -> change.id.value in pointers }
      .forEach(PointerInputChange::consume)
  }

  private fun handlePointerSignal(pointerEvent: PointerEvent) {
    if (!enabled || density <= 0f) {
      finishWheelZoom()
      return
    }
    val scrollDelta =
      pointerEvent.changes.fold(Offset.Zero) { total, change ->
        if (change.isConsumed) total else total + change.scrollDelta
      }
    if (scrollDelta == Offset.Zero) {
      return
    }
    val zoomModified =
      pointerEvent.keyboardModifiers.isCtrlPressed || pointerEvent.keyboardModifiers.isMetaPressed
    if (!zoomModified) {
      finishWheelZoom()
      if (scrollDriver.launchPointerSignalScroll(scrollDelta = scrollDelta, density = density)) {
        onViewportWheelScroll()
        pointerEvent.changes.forEach(PointerInputChange::consume)
      }
      return
    }

    val change = pointerEvent.changes.firstOrNull() ?: return
    val dominantDelta =
      if (abs(scrollDelta.y) >= abs(scrollDelta.x)) scrollDelta.y else scrollDelta.x
    if (!dominantDelta.isFinite() || dominantDelta == 0f) {
      return
    }
    val normalizedDelta = normalizeEditorViewportWheelZoomDelta(dominantDelta)
    val deltaMagnitude = abs(normalizedDelta)
    val elapsed = wheelLastEventMillis?.let { change.uptimeMillis - it } ?: Long.MAX_VALUE
    if (elapsed > WheelBurstGapMs) {
      finishWheelZoom()
    }
    wheelLastEventMillis = change.uptimeMillis
    if (deltaMagnitude <= WheelTailDeltaPx) {
      wheelLowDeltaStreak += 1
      if (wheelLowDeltaStreak >= WheelTailStreakToReset) {
        finishWheelZoom()
        return
      }
    } else {
      wheelLowDeltaStreak = 0
    }
    if (!wheelZoomActive) {
      if (
        deltaMagnitude < WheelModeSwitchMinDeltaPx ||
          !interactionController.beginPointerSignalZoom()
      ) {
        return
      }
      wheelZoomActive = true
    }
    val focalInEditor = geometry.resolveInteractionPosition(change.position)
    if (
      focalInEditor == null ||
        !interactionController.updatePointerSignalZoom(
          focalInEditorPx = focalInEditor,
          normalizedDelta = normalizedDelta,
        )
    ) {
      finishWheelZoom()
      return
    }
    keepWheelZoomAlive()
    pointerEvent.changes.forEach(PointerInputChange::consume)
  }

  private fun keepWheelZoomAlive() {
    wheelZoomTimeoutJob?.cancel()
    wheelZoomTimeoutJob = coroutineScope.launch {
      delay(WheelBurstGapMs)
      finishWheelZoom()
    }
  }

  private fun finishWheelZoom() {
    wheelZoomTimeoutJob?.cancel()
    wheelZoomTimeoutJob = null
    wheelLowDeltaStreak = 0
    wheelLastEventMillis = null
    if (wheelZoomActive) {
      wheelZoomActive = false
      interactionController.endPointerSignalZoom()
    }
  }

  private fun cancelInteraction(clearSuppression: Boolean) {
    val hadPointers = pointers.isNotEmpty() || suppressUntilAllUp
    pointers.clear()
    singlePointerStreams.clear()
    if (hadPointers) {
      interactionController.cancel()
    }
    scrollDriver.cancel()
    finishWheelZoom()
    if (clearSuppression) {
      suppressUntilAllUp = false
    }
  }

  override fun onDetach() {
    cancelInteraction(clearSuppression = true)
    screenPointerSequence.detach(this)
    super.onDetach()
  }
}

internal class EditorScreenPointerSequence {
  private val screenPointers = mutableSetOf<Long>()
  private var listener: EditorScreenPointerListener? = null
  private var pressedMembershipChanged = false

  val hasScreenPointers: Boolean
    get() = screenPointers.isNotEmpty()

  fun attach(listener: EditorScreenPointerListener) {
    this.listener = listener
  }

  fun detach(listener: EditorScreenPointerListener) {
    if (this.listener === listener) {
      this.listener = null
    }
  }

  fun hasForeignTouchPointer(isEditorTouchPointer: (Long) -> Boolean): Boolean =
    screenPointers.any { pointerId ->
      !isEditorTouchPointer(pointerId)
    }

  fun observePressedPointers(pointerEvent: PointerEvent) {
    pointerEvent.changes
      .filter { change -> change.type == PointerType.Touch && change.pressed }
      .forEach { change ->
        pressedMembershipChanged = screenPointers.add(change.id.value) || pressedMembershipChanged
      }
  }

  fun observeReleasedPointers(pointerEvent: PointerEvent) {
    if (pressedMembershipChanged) {
      pressedMembershipChanged = false
      listener?.onScreenPointerMembershipChanged()
    }
    val hadScreenPointers = screenPointers.isNotEmpty()
    pointerEvent.changes
      .filter { change -> change.type == PointerType.Touch && !change.pressed }
      .forEach { change -> screenPointers -= change.id.value }
    if (screenPointers.isEmpty()) {
      if (hadScreenPointers) {
        listener?.onGlobalAllUp()
      }
    }
  }

  fun reset() {
    val shouldNotify = screenPointers.isNotEmpty()
    screenPointers.clear()
    pressedMembershipChanged = false
    if (shouldNotify) {
      listener?.onGlobalAllUp()
    }
  }
}

internal interface EditorScreenPointerListener {
  fun onScreenPointerMembershipChanged()

  fun onGlobalAllUp()
}

internal fun Modifier.observeEditorScreenPointerSequence(
  sequence: EditorScreenPointerSequence
): Modifier = this then EditorScreenPointerObserverElement(sequence)

private data class EditorScreenPointerObserverElement(
  private val sequence: EditorScreenPointerSequence
) : ModifierNodeElement<EditorScreenPointerObserverNode>() {
  override fun create(): EditorScreenPointerObserverNode = EditorScreenPointerObserverNode(sequence)

  override fun update(node: EditorScreenPointerObserverNode) {
    if (node.sequence !== sequence) {
      node.sequence.reset()
      node.sequence = sequence
    }
  }
}

private class EditorScreenPointerObserverNode(var sequence: EditorScreenPointerSequence) :
  Modifier.Node(), PointerInputModifierNode {
  override fun onPointerEvent(pointerEvent: PointerEvent, pass: PointerEventPass, bounds: IntSize) {
    when (pass) {
      PointerEventPass.Initial -> sequence.observePressedPointers(pointerEvent)
      PointerEventPass.Final -> sequence.observeReleasedPointers(pointerEvent)
      PointerEventPass.Main -> Unit
    }
  }

  override fun onCancelPointerInput() {
    sequence.reset()
  }

  override fun onDetach() {
    sequence.reset()
    super.onDetach()
  }
}

private fun PointerEvent.inputModifiers(): InputModifiers {
  val modifiers = keyboardModifiers
  return InputModifiers(
    shift = modifiers.isShiftPressed,
    ctrl = modifiers.isCtrlPressed,
    alt = modifiers.isAltPressed,
    meta = modifiers.isMetaPressed,
  )
}
