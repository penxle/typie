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
import co.typie.ext.ScrollGestureLockHandle
import co.typie.ext.ScrollGestureLockState
import co.typie.platform.isDirectMousePress
import co.typie.platform.isTouchDragPointer
import kotlin.math.abs
import kotlin.math.min
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

private const val EditorTapSlopDp = 8f
private const val WheelBurstGapMs = 56L

internal fun Modifier.editorInteractions(
  interactionController: EditorInteractionController,
  geometry: EditorInteractionGeometry,
  screenPointerSequence: EditorScreenPointerSequence,
  platformIndirectScaleBridge: EditorPlatformIndirectScaleBridge,
  scrollGestureLockState: ScrollGestureLockState,
  scrollableState: Scrollable2DState? = null,
  nestedScrollDispatcher: NestedScrollDispatcher? = null,
  flingBehavior: FlingBehavior? = null,
  touchSlop: Float = 0f,
  maximumFlingVelocity: Float = Float.MAX_VALUE,
  density: Float,
  enabled: Boolean = true,
  onEditorPointerInput: () -> Unit = {},
  onNestedScrollCancel: () -> Unit = {},
): Modifier =
  this then
    EditorInteractionsElement(
      interactionController = interactionController,
      geometry = geometry,
      screenPointerSequence = screenPointerSequence,
      platformIndirectScaleBridge = platformIndirectScaleBridge,
      scrollGestureLockState = scrollGestureLockState,
      scrollableState = scrollableState,
      nestedScrollDispatcher = nestedScrollDispatcher,
      flingBehavior = flingBehavior,
      touchSlop = touchSlop,
      maximumFlingVelocity = maximumFlingVelocity,
      density = density,
      enabled = enabled,
      onEditorPointerInput = onEditorPointerInput,
      onNestedScrollCancel = onNestedScrollCancel,
    )

private data class EditorInteractionsElement(
  private val interactionController: EditorInteractionController,
  private val geometry: EditorInteractionGeometry,
  private val screenPointerSequence: EditorScreenPointerSequence,
  private val platformIndirectScaleBridge: EditorPlatformIndirectScaleBridge,
  private val scrollGestureLockState: ScrollGestureLockState,
  private val scrollableState: Scrollable2DState?,
  private val nestedScrollDispatcher: NestedScrollDispatcher?,
  private val flingBehavior: FlingBehavior?,
  private val touchSlop: Float,
  private val maximumFlingVelocity: Float,
  private val density: Float,
  private val enabled: Boolean,
  private val onEditorPointerInput: () -> Unit,
  private val onNestedScrollCancel: () -> Unit,
) : ModifierNodeElement<EditorInteractionsNode>() {
  override fun create(): EditorInteractionsNode =
    EditorInteractionsNode(
      interactionController = interactionController,
      geometry = geometry,
      screenPointerSequence = screenPointerSequence,
      platformIndirectScaleBridge = platformIndirectScaleBridge,
      scrollGestureLockState = scrollGestureLockState,
      scrollableState = scrollableState,
      nestedScrollDispatcher = nestedScrollDispatcher,
      flingBehavior = flingBehavior,
      touchSlop = touchSlop,
      maximumFlingVelocity = maximumFlingVelocity,
      density = density,
      enabled = enabled,
      onEditorPointerInput = onEditorPointerInput,
      onNestedScrollCancel = onNestedScrollCancel,
    )

  override fun update(node: EditorInteractionsNode) {
    node.update(
      interactionController = interactionController,
      geometry = geometry,
      screenPointerSequence = screenPointerSequence,
      platformIndirectScaleBridge = platformIndirectScaleBridge,
      scrollGestureLockState = scrollGestureLockState,
      scrollableState = scrollableState,
      nestedScrollDispatcher = nestedScrollDispatcher,
      flingBehavior = flingBehavior,
      touchSlop = touchSlop,
      maximumFlingVelocity = maximumFlingVelocity,
      density = density,
      enabled = enabled,
      onEditorPointerInput = onEditorPointerInput,
      onNestedScrollCancel = onNestedScrollCancel,
    )
  }
}

private class EditorInteractionsNode(
  var interactionController: EditorInteractionController,
  var geometry: EditorInteractionGeometry,
  var screenPointerSequence: EditorScreenPointerSequence,
  var platformIndirectScaleBridge: EditorPlatformIndirectScaleBridge,
  var scrollGestureLockState: ScrollGestureLockState,
  var scrollableState: Scrollable2DState?,
  var nestedScrollDispatcher: NestedScrollDispatcher?,
  var flingBehavior: FlingBehavior?,
  var touchSlop: Float,
  var maximumFlingVelocity: Float,
  var density: Float,
  var enabled: Boolean,
  var onEditorPointerInput: () -> Unit,
  var onNestedScrollCancel: () -> Unit,
) :
  Modifier.Node(),
  PointerInputModifierNode,
  SemanticsModifierNode,
  EditorScreenPointerListener,
  EditorPlatformIndirectScaleOwner {
  private val pointers = mutableMapOf<Long, PointerType>()
  private val singlePointerStreams = mutableSetOf<Long>()
  private var suppressUntilAllUp = false
  private var wheelLastEventMillis: Long? = null
  private var wheelZoomActive = false
  private var wheelZoomTimeoutJob: Job? = null
  private var scaleZoomActive = false
  private var physicalSequenceYieldedToIndirectInput = false
  private var physicalDragLockHandle: ScrollGestureLockHandle? = null
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
    platformIndirectScaleBridge.attach(this)
  }

  fun update(
    interactionController: EditorInteractionController,
    geometry: EditorInteractionGeometry,
    screenPointerSequence: EditorScreenPointerSequence,
    platformIndirectScaleBridge: EditorPlatformIndirectScaleBridge,
    scrollGestureLockState: ScrollGestureLockState,
    scrollableState: Scrollable2DState?,
    nestedScrollDispatcher: NestedScrollDispatcher?,
    flingBehavior: FlingBehavior?,
    touchSlop: Float,
    maximumFlingVelocity: Float,
    density: Float,
    enabled: Boolean,
    onEditorPointerInput: () -> Unit,
    onNestedScrollCancel: () -> Unit,
  ) {
    val inputOwnerChanged =
      this.interactionController !== interactionController ||
        this.geometry !== geometry ||
        this.screenPointerSequence !== screenPointerSequence ||
        this.platformIndirectScaleBridge !== platformIndirectScaleBridge ||
        this.scrollGestureLockState !== scrollGestureLockState ||
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
    if (this.platformIndirectScaleBridge !== platformIndirectScaleBridge) {
      this.platformIndirectScaleBridge.detach(this)
      platformIndirectScaleBridge.attach(this)
    }
    this.interactionController = interactionController
    this.geometry = geometry
    this.screenPointerSequence = screenPointerSequence
    this.platformIndirectScaleBridge = platformIndirectScaleBridge
    this.scrollGestureLockState = scrollGestureLockState
    this.scrollableState = scrollableState
    this.nestedScrollDispatcher = nestedScrollDispatcher
    this.flingBehavior = flingBehavior
    this.touchSlop = touchSlop
    this.maximumFlingVelocity = maximumFlingVelocity
    this.density = density
    this.enabled = enabled
    this.onEditorPointerInput = onEditorPointerInput
    this.onNestedScrollCancel = onNestedScrollCancel
  }

  override fun onPointerEvent(pointerEvent: PointerEvent, pass: PointerEventPass, bounds: IntSize) {
    if (routePointerPass(pointerEvent = pointerEvent, pass = pass)) {
      return
    }
    if (!enabled || density <= 0f) {
      cancelInteraction(clearSuppression = true)
      return
    }
    if (pointerEvent.changes.any { change -> change.isUnconsumedDirectDown(pointerEvent) }) {
      onEditorPointerInput()
    }

    interactionController.updateTapSlop(tapSlopPx = EditorTapSlopDp * density)
    interactionController.updateColumnResizeSlop(
      dragSlopPx = min(touchSlop, EditorTapSlopDp * density)
    )
    registerPointerDowns(pointerEvent)
    if (
      scaleZoomActive &&
        pointers.isNotEmpty() &&
        pointers.values.all { pointerType -> pointerType == PointerType.Mouse }
    ) {
      suppressUntilAllUp = true
      physicalSequenceYieldedToIndirectInput = true
    }
    if (physicalSequenceYieldedToIndirectInput) {
      consumeEditorChanges(pointerEvent)
      finishReleasedPointers(pointerEvent)
      return
    }
    if (pointers.isEmpty()) {
      return
    }
    finishWheelZoom()
    finishScaleZoom()
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

  private fun routePointerPass(pointerEvent: PointerEvent, pass: PointerEventPass): Boolean {
    if (pointerEvent.type.isIndirectPointerEvent()) {
      if (pass == PointerEventPass.Initial) {
        handleIndirectPointerEvent(pointerEvent)
      }
      return true
    }

    val hasFreshDown = pointerEvent.changes.any { change -> change.isDirectDown(pointerEvent) }
    // Track membership before the screen observer's Final pass, but let shared overlay siblings
    // claim direct gestures during Main before admitting a new editor sequence.
    return when (pass) {
      PointerEventPass.Initial -> true
      PointerEventPass.Main -> {
        if (hasFreshDown && enabled && density > 0f) {
          registerPointerDowns(pointerEvent)
        }
        hasFreshDown
      }
      PointerEventPass.Final -> {
        if (hasFreshDown) {
          discardConsumedPointerDowns(pointerEvent)
        }
        !hasFreshDown
      }
    }
  }

  override fun onCancelPointerInput() {
    cancelInteraction(clearSuppression = true)
  }

  override fun onScreenPointerMembershipChanged() {
    if (
      (pointers.isEmpty() && interactionController.interactionMode == EditorInteractionMode.Idle) ||
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
    physicalSequenceYieldedToIndirectInput = false
    releasePhysicalDragLock()
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
      .filter { it.isUnconsumedDirectDown(pointerEvent) }
      .forEach { change ->
        pointers[change.id.value] = change.type
        if (
          physicalDragLockHandle == null &&
            change.type == PointerType.Mouse &&
            change.type.isTouchDragPointer()
        ) {
          physicalDragLockHandle = scrollGestureLockState.acquire()
        }
      }
  }

  private fun discardConsumedPointerDowns(pointerEvent: PointerEvent) {
    var pointerRemoved = false
    pointerEvent.changes
      .filter { change -> change.isDirectDown(pointerEvent) && change.isConsumed }
      .forEach { change ->
        if (pointers.remove(change.id.value) != null) {
          pointerRemoved = true
        }
        singlePointerStreams -= change.id.value
      }
    if (!pointerRemoved) {
      return
    }
    if (!hasPhysicalDragPointer()) {
      releasePhysicalDragLock()
    }
    onScreenPointerMembershipChanged()
  }

  private fun isEditorTouchPointer(pointerId: Long): Boolean =
    pointers[pointerId] == PointerType.Touch

  private fun pressedTouchChanges(pointerEvent: PointerEvent): List<PointerInputChange> =
    pointerEvent.changes.filter { change ->
      change.pressed && pointers[change.id.value] == PointerType.Touch
    }

  private fun forwardSinglePointerChanges(pointerEvent: PointerEvent) {
    pointerEvent.changes
      .filter { it.isUnconsumedDirectDown(pointerEvent) }
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
            touchPanDriver = if (change.type.isTouchDragPointer()) scrollDriver else null,
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
      physicalSequenceYieldedToIndirectInput = false
    }
    if (!hasPhysicalDragPointer()) {
      releasePhysicalDragLock()
    }
  }

  private fun hasPhysicalDragPointer(): Boolean =
    pointers.values.any { pointerType ->
      pointerType == PointerType.Mouse && pointerType.isTouchDragPointer()
    }

  private fun cancelAndSuppress(pointerEvent: PointerEvent) {
    physicalSequenceYieldedToIndirectInput = false
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
    physicalSequenceYieldedToIndirectInput = false
    finishWheelZoom()
    finishScaleZoom()
    if (hadEditorInteraction) {
      interactionController.cancel()
    }
    scrollDriver.cancel()
  }

  private fun consumeEditorChanges(pointerEvent: PointerEvent) {
    pointerEvent.changes
      .filter { change -> change.id.value in pointers }
      .forEach(PointerInputChange::consume)
  }

  private fun handleIndirectPointerEvent(pointerEvent: PointerEvent): Boolean {
    when (pointerEvent.type) {
      PointerEventType.Scroll -> {
        finishScaleZoom()
        handlePointerSignal(pointerEvent)
      }
      PointerEventType.PanStart,
      PointerEventType.PanMove,
      PointerEventType.PanEnd -> {
        finishWheelZoom()
        finishScaleZoom()
        handleTrackpadPan(pointerEvent)
      }
      PointerEventType.ScaleStart,
      PointerEventType.ScaleChange,
      PointerEventType.ScaleEnd -> {
        finishWheelZoom()
        handleScale(pointerEvent)
      }
      else -> return false
    }
    return true
  }

  private fun handlePointerSignal(pointerEvent: PointerEvent) {
    if (!enabled || density <= 0f) {
      finishWheelZoom()
      return
    }
    if (screenPointerSequence.hasScreenPointers) {
      finishWheelZoom()
      pointerEvent.changes.forEach(PointerInputChange::consume)
      return
    }
    if (
      pointers.isNotEmpty() &&
        !physicalSequenceYieldedToIndirectInput &&
        !yieldPendingPhysicalPointerToIndirectInput()
    ) {
      finishWheelZoom()
      pointerEvent.changes.forEach(PointerInputChange::consume)
      return
    }
    val scrollDelta =
      pointerEvent.changes.fold(Offset.Zero) { total, change ->
        if (change.isConsumed) total else total + change.scrollDelta
      }
    if (scrollDelta == Offset.Zero) {
      return
    }
    val zoomModified = pointerEvent.keyboardModifiers.isEditorIndirectZoomModifierPressed()
    if (!zoomModified) {
      finishWheelZoom()
      if (scrollDriver.launchPointerSignalScroll(scrollDelta = scrollDelta, density = density)) {
        onEditorPointerInput()
        pointerEvent.changes.forEach(PointerInputChange::consume)
      }
      return
    }

    handleModifiedPointerScroll(pointerEvent = pointerEvent, scrollDelta = scrollDelta)
  }

  private fun handleModifiedPointerScroll(pointerEvent: PointerEvent, scrollDelta: Offset) {
    val change = pointerEvent.changes.firstOrNull { candidate -> !candidate.isConsumed } ?: return
    val dominantDelta =
      if (abs(scrollDelta.y) >= abs(scrollDelta.x)) scrollDelta.y else scrollDelta.x
    if (!dominantDelta.isFinite() || dominantDelta == 0f) {
      return
    }
    val normalizedDelta = normalizeEditorViewportWheelZoomDelta(dominantDelta)
    val elapsed = wheelLastEventMillis?.let { change.uptimeMillis - it } ?: Long.MAX_VALUE
    if (elapsed > WheelBurstGapMs) {
      finishWheelZoom()
    }
    wheelLastEventMillis = change.uptimeMillis
    if (!wheelZoomActive) {
      if (!interactionController.beginIndirectZoom()) {
        return
      }
      wheelZoomActive = true
    }
    val focalInRoot = positionInRoot(change.position)
    if (
      !interactionController.updateIndirectScrollZoom(
        focalInRootPx = focalInRoot,
        normalizedDelta = normalizedDelta,
      )
    ) {
      finishWheelZoom()
      return
    }
    keepWheelZoomAlive()
    onEditorPointerInput()
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
    wheelLastEventMillis = null
    if (wheelZoomActive) {
      wheelZoomActive = false
      interactionController.endIndirectZoom()
    }
  }

  private fun handleTrackpadPan(pointerEvent: PointerEvent) {
    if (!enabled || density <= 0f) {
      return
    }
    if (!canAcceptIndirectInput() && !yieldPendingPhysicalPointerToIndirectInput()) {
      pointerEvent.changes.forEach(PointerInputChange::consume)
      return
    }
    if (pointerEvent.type == PointerEventType.PanMove) {
      val panOffset =
        pointerEvent.changes.firstNotNullOfOrNull { change ->
          change.panOffset.takeIf { offset -> !change.isConsumed && offset.isUsablePanOffset() }
        }
      if (panOffset != null && scrollDriver.launchTrackpadPan(panOffset)) {
        onEditorPointerInput()
      }
    }
    pointerEvent.changes.forEach(PointerInputChange::consume)
  }

  private fun handleScale(pointerEvent: PointerEvent) {
    if (!enabled || density <= 0f) {
      finishScaleZoom()
      return
    }
    when (pointerEvent.type) {
      PointerEventType.ScaleStart -> {
        beginIndirectScale()
      }
      PointerEventType.ScaleChange -> {
        if (!canAcceptIndirectInput()) {
          finishScaleZoom()
          pointerEvent.changes.forEach(PointerInputChange::consume)
          return
        }
        val change =
          if (scaleZoomActive) {
            pointerEvent.changes.firstOrNull { candidate ->
              !candidate.isConsumed && candidate.scaleFactor.isUsableScaleChange()
            }
          } else {
            null
          }
        if (change != null) {
          updateIndirectScale(
            focalInRootPx = positionInRoot(change.position),
            scaleFactor = change.scaleFactor,
          )
        }
      }
      PointerEventType.ScaleEnd -> endIndirectScale()
    }
    pointerEvent.changes.forEach(PointerInputChange::consume)
  }

  private fun canAcceptIndirectInput(): Boolean =
    enabled &&
      density > 0f &&
      (pointers.isEmpty() || physicalSequenceYieldedToIndirectInput) &&
      !screenPointerSequence.hasScreenPointers

  override fun beginIndirectScale(): Boolean {
    finishWheelZoom()
    if (scaleZoomActive) {
      return false
    }
    if (!canAcceptIndirectInput() && !yieldPendingPhysicalPointerToIndirectInput()) {
      return false
    }
    scaleZoomActive = interactionController.beginIndirectZoom()
    return scaleZoomActive
  }

  override fun updateIndirectScale(focalInRootPx: Offset, scaleFactor: Float): Boolean {
    if (!scaleZoomActive || !canAcceptIndirectInput()) {
      finishScaleZoom()
      return false
    }
    if (
      !interactionController.updateIndirectScaleZoom(
        focalInRootPx = focalInRootPx,
        scaleFactor = scaleFactor,
      )
    ) {
      finishScaleZoom()
      return false
    }
    onEditorPointerInput()
    return true
  }

  override fun endIndirectScale() {
    finishScaleZoom()
  }

  private fun finishScaleZoom() {
    if (scaleZoomActive) {
      scaleZoomActive = false
      interactionController.endIndirectZoom()
    }
  }

  private fun yieldPendingPhysicalPointerToIndirectInput(): Boolean {
    if (
      screenPointerSequence.hasScreenPointers ||
        pointers.isEmpty() ||
        pointers.values.any { pointerType -> pointerType != PointerType.Mouse } ||
        !interactionController.cancelPendingPointerForIndirectInput()
    ) {
      return false
    }
    singlePointerStreams.clear()
    scrollDriver.cancel()
    suppressUntilAllUp = true
    physicalSequenceYieldedToIndirectInput = true
    return true
  }

  private fun releasePhysicalDragLock() {
    physicalDragLockHandle?.release()
    physicalDragLockHandle = null
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
    finishScaleZoom()
    releasePhysicalDragLock()
    physicalSequenceYieldedToIndirectInput = false
    if (clearSuppression) {
      suppressUntilAllUp = false
    }
  }

  override fun onDetach() {
    cancelInteraction(clearSuppression = true)
    screenPointerSequence.detach(this)
    platformIndirectScaleBridge.detach(this)
    super.onDetach()
  }
}

private fun Offset.isUsablePanOffset(): Boolean {
  if (this == Offset.Zero) {
    return false
  }
  return x.isFinite() && y.isFinite()
}

private fun Float.isUsableScaleChange(): Boolean = isFinite() && this > 0f && this != 1f

private fun PointerEventType.isIndirectPointerEvent(): Boolean =
  this == PointerEventType.Scroll ||
    this == PointerEventType.PanStart ||
    this == PointerEventType.PanMove ||
    this == PointerEventType.PanEnd ||
    this == PointerEventType.ScaleStart ||
    this == PointerEventType.ScaleChange ||
    this == PointerEventType.ScaleEnd

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

private fun PointerInputChange.isDirectDown(pointerEvent: PointerEvent): Boolean =
  pressed && !previousPressed && (type != PointerType.Mouse || pointerEvent.isDirectMousePress())

private fun PointerInputChange.isUnconsumedDirectDown(pointerEvent: PointerEvent): Boolean =
  isDirectDown(pointerEvent) && !isConsumed

private fun PointerEvent.inputModifiers(): InputModifiers {
  val modifiers = keyboardModifiers
  return InputModifiers(
    shift = modifiers.isShiftPressed,
    ctrl = modifiers.isCtrlPressed,
    alt = modifiers.isAltPressed,
    meta = modifiers.isMetaPressed,
  )
}
