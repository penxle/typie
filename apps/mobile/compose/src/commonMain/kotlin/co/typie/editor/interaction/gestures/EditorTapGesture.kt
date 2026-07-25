package co.typie.editor.interaction.gestures

import androidx.compose.ui.geometry.Offset
import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.ext.isCollapsed
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.InputModifiers
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionPointUnit
import co.typie.editor.interaction.EditorGestureContext
import co.typie.editor.interaction.isViewportZooming
import co.typie.editor.interaction.semantics.EditorInteractiveTapResult
import co.typie.editor.interaction.sessions.EditorDoubleTapDragSession
import co.typie.platform.Platform

private const val EditorTapDownDelayMillis = 100L
private const val EditorTapTimerDelayMillis = 150L
internal const val EditorTapDispatchDelayMillis =
  EditorTapDownDelayMillis + EditorTapTimerDelayMillis

internal const val EditorConsecutiveTapMaxIntervalMillis = 300L
private const val ConsecutiveTapMaxDistanceDp = 20f

internal data class EditorCompletedTap(
  val count: Int,
  val contentAlreadyDispatched: Boolean,
  val recordInHistory: Boolean,
)

internal class EditorTapGesture(
  private var tapSlopPx: Float,
  private val densityProvider: () -> Float,
  private val consecutiveTapMaxIntervalMillis: Long = EditorConsecutiveTapMaxIntervalMillis,
) {
  private var activeTap: ActiveTap? = null
  private var tapHistory: TapHistory? = null
  private var pendingPresentation: PendingTapPresentation? = null
  private var nextPresentationGeneration = 0L

  val hasActivePointer: Boolean
    get() = activeTap != null

  val activePosition: Offset?
    get() = activeTap?.downPosition

  val inputModifiersForActivePointer: InputModifiers
    get() = activeTap?.inputModifiers ?: InputModifiers()

  val shouldOpenContextMenuForActivePointer: Boolean
    get() = activeTap?.contextMenuWasVisibleAtPointerDown == false

  val tapCountForActivePointer: Int?
    get() = activeTap?.count

  val canDispatchTapTimer: Boolean
    get() = activeTap?.phase == ActiveTapPhase.Pending

  fun updateTapSlop(tapSlopPx: Float) {
    this.tapSlopPx = tapSlopPx
  }

  fun startActivePointer(
    pointerId: Long,
    position: Offset,
    nowMillis: Long,
    inputModifiers: InputModifiers = InputModifiers(),
    contextMenuWasVisibleAtPointerDown: Boolean = false,
  ) {
    activeTap =
      ActiveTap(
        pointerId = pointerId,
        downPosition = position,
        inputModifiers = inputModifiers,
        count = nextTapCount(position = position, nowMillis = nowMillis),
        contextMenuWasVisibleAtPointerDown = contextMenuWasVisibleAtPointerDown,
      )
  }

  fun onPointerMove(pointerId: Long, position: Offset): Boolean {
    val tap = activeTap
    if (tap == null || tap.pointerId != pointerId) {
      return false
    }
    if ((position - tap.downPosition).getDistance() > tapSlopPx) {
      tap.phase = ActiveTapPhase.Moved
      return true
    }
    return false
  }

  fun markTapDispatched() {
    activeTap?.let { tap ->
      if (tap.phase == ActiveTapPhase.Pending) {
        tap.phase = ActiveTapPhase.Dispatched
      }
    }
  }

  fun shouldConsumePointerUp(pointerId: Long, canFinish: Boolean): Boolean {
    val tap = activeTap ?: return false
    return canFinish && tap.pointerId == pointerId && tap.phase != ActiveTapPhase.Moved
  }

  fun onPointerUp(
    pointerId: Long,
    position: Offset,
    nowMillis: Long,
    canFinish: Boolean = true,
  ): EditorCompletedTap? {
    val tap = activeTap
    if (tap == null || tap.pointerId != pointerId) {
      return null
    }
    if (!canFinish) {
      clearActivePointer()
      return null
    }

    if (tap.phase == ActiveTapPhase.Moved) {
      clearActivePointer()
      return null
    }
    val completed =
      EditorCompletedTap(
        count = tap.semanticCount,
        contentAlreadyDispatched = tap.phase == ActiveTapPhase.Dispatched,
        recordInHistory = tap.recordInHistory,
      )
    if (completed.recordInHistory) {
      recordTap(nowMillis = nowMillis, position = position, clickCount = completed.count)
    }
    clearActivePointer()
    return completed
  }

  fun cancelActivePointerStream(): Boolean {
    val hadActivePointer = activeTap != null
    clearActivePointer()
    return hadActivePointer
  }

  fun reset() {
    clearActivePointer()
    clearTapHistory()
    pendingPresentation = null
  }

  private fun clearActivePointer() {
    activeTap = null
  }

  fun recordTap(nowMillis: Long, position: Offset, clickCount: Int) {
    if (clickCount >= 3) {
      clearTapHistory()
      return
    }
    tapHistory = TapHistory(completedAtMillis = nowMillis, position = position, count = clickCount)
  }

  fun clearTapHistory() {
    tapHistory = null
  }

  fun prepareActiveTapForEditingPromotion() {
    activeTap?.let { tap ->
      tap.semanticCount = 1
      tap.recordInHistory = false
      tap.preservePresentationAcrossEditingPromotion = true
      if (tap.phase == ActiveTapPhase.Pending) {
        tap.phase = ActiveTapPhase.Dispatched
      }
    }
  }

  fun shouldPreservePresentationAfterEditingPromotion(): Boolean =
    activeTap?.preservePresentationAcrossEditingPromotion == true

  fun beginPendingPresentation(
    editor: Editor,
    expectedEditing: Boolean,
    tapCount: Int,
    showReadingHint: Boolean,
    context: EditorGestureContext,
  ): Long {
    cancelPendingPresentation(context = context)
    val generation = ++nextPresentationGeneration
    pendingPresentation =
      PendingTapPresentation(
        generation = generation,
        editor = editor,
        expectedEditing = expectedEditing,
        tapCount = tapCount,
        showReadingHint = showReadingHint,
      )
    return generation
  }

  fun markPendingSelectionPublished(
    generation: Long,
    snapshot: EditorState,
    showContextMenu: Boolean,
    context: EditorGestureContext,
  ) {
    val pending = pendingPresentation
    if (pending == null || pending.generation != generation) {
      return
    }
    if (pending.editor.state.selection != snapshot.selection) {
      cancelPendingPresentation(context = context)
      return
    }
    pending.selectionPublished = true
    pending.committedSelection = snapshot.selection
    pending.showContextMenu = showContextMenu
    deliverPendingPresentationIfReady(context = context)
  }

  fun confirmPendingPresentationAfterPointerUp(
    completedTap: EditorCompletedTap,
    context: EditorGestureContext,
  ) {
    val pending = pendingPresentation ?: return
    if (pending.tapCount != completedTap.count) {
      return
    }
    val generation = pending.generation
    if (completedTap.count >= 3) {
      markPendingSequenceConfirmed(generation = generation, context = context)
      return
    }
    context.effects.scheduleTapSequenceConfirmation {
      markPendingSequenceConfirmed(generation = generation, context = context)
    }
  }

  fun cancelPendingPresentation(context: EditorGestureContext) {
    pendingPresentation = null
    context.effects.cancelTapSequenceConfirmation()
  }

  private fun markPendingSequenceConfirmed(generation: Long, context: EditorGestureContext) {
    val pending = pendingPresentation
    if (pending == null || pending.generation != generation) {
      return
    }
    pending.sequenceConfirmed = true
    deliverPendingPresentationIfReady(context = context)
  }

  private fun deliverPendingPresentationIfReady(context: EditorGestureContext) {
    val pending = pendingPresentation ?: return
    if (!pending.selectionPublished || !pending.sequenceConfirmed) {
      return
    }
    if (
      context.editor !== pending.editor ||
        context.editing != pending.expectedEditing ||
        pending.editor.state.selection != pending.committedSelection
    ) {
      cancelPendingPresentation(context = context)
      return
    }
    pendingPresentation = null
    if (pending.showReadingHint && !context.readOnly) {
      context.effects.showReadingEditHint()
    }
    if (pending.showContextMenu) {
      context.semantics.contextMenu.show(pending.editor.state)
    }
  }

  fun nextTapCount(position: Offset, nowMillis: Long): Int {
    val previousTap = tapHistory ?: return 1
    return if (
      nowMillis - previousTap.completedAtMillis < consecutiveTapMaxIntervalMillis &&
        (position - previousTap.position).getDistance() <
          ConsecutiveTapMaxDistanceDp * densityProvider()
    ) {
      previousTap.count + 1
    } else {
      1
    }
  }

  private class ActiveTap(
    val pointerId: Long,
    val downPosition: Offset,
    val inputModifiers: InputModifiers,
    val count: Int,
    val contextMenuWasVisibleAtPointerDown: Boolean,
    var phase: ActiveTapPhase = ActiveTapPhase.Pending,
    var semanticCount: Int = count,
    var recordInHistory: Boolean = true,
    var preservePresentationAcrossEditingPromotion: Boolean = false,
  )

  private enum class ActiveTapPhase {
    Pending,
    Dispatched,
    Moved,
  }

  private data class TapHistory(val completedAtMillis: Long, val position: Offset, val count: Int)

  private data class PendingTapPresentation(
    val generation: Long,
    val editor: Editor,
    val expectedEditing: Boolean,
    val tapCount: Int,
    val showReadingHint: Boolean,
    var selectionPublished: Boolean = false,
    var committedSelection: Selection? = null,
    var showContextMenu: Boolean = false,
    var sequenceConfirmed: Boolean = false,
  )
}

internal fun EditorTapGesture.handlePointerDown(
  pointerId: Long,
  position: Offset,
  nowMillis: Long,
  tapEnabled: Boolean,
  inputModifiers: InputModifiers,
  doubleTapDrag: EditorDoubleTapDragSession,
  context: EditorGestureContext,
): Boolean {
  if (!tapEnabled || context.mode.isViewportZooming) {
    return false
  }

  startActivePointer(
    pointerId = pointerId,
    position = position,
    nowMillis = nowMillis,
    inputModifiers = inputModifiers,
    contextMenuWasVisibleAtPointerDown = context.semantics.contextMenu.visible,
  )
  cancelPendingPresentation(context = context)
  context.semantics.contextMenu.hide()
  if (!context.editing) {
    if (tapCountForActivePointer == 2 && context.doubleTapToEditEnabled) {
      prepareActiveTapForEditingPromotion()
      context.effects.cancelTapDispatch()
      val handled =
        dispatchReadingActivation(
          position = position,
          inputModifiers = inputModifiers,
          shouldOpenContextMenu = shouldOpenContextMenuForActivePointer,
          doubleTapDrag = doubleTapDrag,
          context = context,
        )
      clearTapHistory()
      return handled
    }
    return false
  }
  if (tapCountForActivePointer == 2) {
    markTapDispatched()
    context.effects.cancelTapDispatch()
    dispatchEditableTap(
      position = position,
      clickCount = 2,
      inputModifiers = inputModifiers,
      shouldOpenContextMenu = shouldOpenContextMenuForActivePointer,
      doubleTapDrag = doubleTapDrag,
      context = context,
    ) {
      doubleTapDrag.prepareForDrag(position = position, tap = this, context = context)
    }
    return true
  }

  context.effects.scheduleTapDispatch(dispatchAtMillis = nowMillis + EditorTapDispatchDelayMillis)
  return false
}

internal fun EditorTapGesture.trackPointerMove(
  pointerId: Long,
  position: Offset,
  context: EditorGestureContext,
): Boolean {
  if (onPointerMove(pointerId = pointerId, position = position)) {
    context.effects.cancelTapDispatch()
    return true
  }
  return false
}

internal fun EditorTapGesture.handlePointerUp(
  pointerId: Long,
  position: Offset,
  nowMillis: Long,
  doubleTapDrag: EditorDoubleTapDragSession,
  context: EditorGestureContext,
): Boolean {
  val canFinishTap = !context.mode.isViewportZooming && !doubleTapDrag.dragging
  val shouldConsumeTap = shouldConsumePointerUp(pointerId = pointerId, canFinish = canFinishTap)
  val inputModifiers = inputModifiersForActivePointer
  val shouldOpenContextMenu = shouldOpenContextMenuForActivePointer
  val completedTap =
    onPointerUp(
      pointerId = pointerId,
      position = position,
      nowMillis = nowMillis,
      canFinish = canFinishTap,
    )
  if (!canFinishTap) {
    context.effects.cancelTapDispatch()
  }
  if (!context.editing) {
    completedTap?.let { completed ->
      if (completed.contentAlreadyDispatched) {
        confirmPendingPresentationAfterPointerUp(completedTap = completed, context = context)
        return@let
      }
      dispatchReadingTap(
        position = position,
        clickCount = completed.count,
        inputModifiers = inputModifiers,
        shouldOpenContextMenu = shouldOpenContextMenu,
        doubleTapDrag = doubleTapDrag,
        context = context,
      )
      confirmPendingPresentationAfterPointerUp(completedTap = completed, context = context)
    }
    return shouldConsumeTap
  }
  completedTap?.let { completed ->
    if (!completed.contentAlreadyDispatched) {
      dispatchEditableTap(
        position = position,
        clickCount = completed.count,
        inputModifiers = inputModifiers,
        shouldOpenContextMenu = shouldOpenContextMenu,
        doubleTapDrag = doubleTapDrag,
        context = context,
        beforeLaunch = {},
      )
    }
    confirmPendingPresentationAfterPointerUp(completedTap = completed, context = context)
  }
  return shouldConsumeTap
}

internal fun EditorTapGesture.handleTapTimer(
  nowMillis: Long,
  doubleTapDrag: EditorDoubleTapDragSession,
  context: EditorGestureContext,
) {
  if (!context.editing) {
    return
  }
  val position = activePosition ?: return
  val inputModifiers = inputModifiersForActivePointer
  if (!canDispatchTapTimer) {
    return
  }
  val clickCount = tapCountForActivePointer ?: return
  val point = context.geometry.resolvePoint(positionInNode = position)
  val hitSelection =
    point != null &&
      point.page >= 0 &&
      context.editor.selectionHitTest(page = point.page, x = point.x, y = point.y)
  if (clickCount == 1 && context.platform != Platform.Android && hitSelection) {
    markTapDispatched()
    val snapshot = context.editor.state
    val generation =
      beginPendingPresentation(
        editor = context.editor,
        expectedEditing = true,
        tapCount = clickCount,
        showReadingHint = false,
        context = context,
      )
    markPendingSelectionPublished(
      generation = generation,
      snapshot = snapshot,
      showContextMenu = shouldOpenContextMenuForActivePointer && !snapshot.selection.isCollapsed(),
      context = context,
    )
    return
  }
  if (clickCount == 1 && !context.editor.selection.isCollapsed()) {
    return
  }
  markTapDispatched()
  dispatchEditableTap(
    position = position,
    clickCount = clickCount,
    inputModifiers = inputModifiers,
    shouldOpenContextMenu = shouldOpenContextMenuForActivePointer,
    doubleTapDrag = doubleTapDrag,
    context = context,
    beforeLaunch = {},
  )
}

private fun EditorTapGesture.dispatchReadingTap(
  position: Offset,
  clickCount: Int,
  inputModifiers: InputModifiers,
  shouldOpenContextMenu: Boolean,
  doubleTapDrag: EditorDoubleTapDragSession,
  context: EditorGestureContext,
): Boolean {
  val point =
    context.geometry.resolvePoint(positionInNode = position)
      ?: run {
        clearTapHistory()
        return false
      }
  if (context.mode.isViewportZooming || point.page < 0) {
    clearTapHistory()
    return false
  }
  val editor = context.editor
  when (
    context.semantics.interactiveHit.handleTap(
      editor = editor,
      point = point,
      editing = false,
      readOnly = context.readOnly,
    )
  ) {
    EditorInteractiveTapResult.HandledViewAction,
    EditorInteractiveTapResult.BlockedMutation -> {
      clearTapHistory()
      cancelPendingPresentation(context = context)
      return true
    }
    EditorInteractiveTapResult.None -> Unit
  }
  if (clickCount != 1) {
    clearTapHistory()
    return false
  }
  if (!context.doubleTapToEditEnabled) {
    return dispatchReadingActivation(
      position = position,
      inputModifiers = inputModifiers,
      shouldOpenContextMenu = shouldOpenContextMenu,
      doubleTapDrag = doubleTapDrag,
      context = context,
    )
  }

  val generation =
    beginPendingPresentation(
      editor = editor,
      expectedEditing = false,
      tapCount = clickCount,
      showReadingHint = true,
      context = context,
    )
  var candidateSnapshot: EditorState? = null
  context.semantics.pointSelection.launchCursorMove(
    editor = editor,
    point = point,
    beforeCommit = { snapshot -> candidateSnapshot = snapshot },
    afterDispatch = { dispatched ->
      val snapshot = candidateSnapshot
      if (dispatched && snapshot != null) {
        markPendingSelectionPublished(
          generation = generation,
          snapshot = snapshot,
          showContextMenu = shouldOpenContextMenu && !snapshot.selection.isCollapsed(),
          context = context,
        )
      } else {
        cancelPendingPresentation(context = context)
      }
    },
  )
  return true
}

private fun EditorTapGesture.dispatchReadingActivation(
  position: Offset,
  inputModifiers: InputModifiers,
  shouldOpenContextMenu: Boolean,
  doubleTapDrag: EditorDoubleTapDragSession,
  context: EditorGestureContext,
): Boolean {
  val point =
    context.geometry.resolvePoint(positionInNode = position)
      ?: run {
        clearTapHistory()
        return false
      }
  if (context.mode.isViewportZooming || point.page < 0) {
    clearTapHistory()
    return false
  }
  val editor = context.editor
  when (
    context.semantics.interactiveHit.handleTap(
      editor = editor,
      point = point,
      editing = false,
      readOnly = context.readOnly,
    )
  ) {
    EditorInteractiveTapResult.HandledViewAction,
    EditorInteractiveTapResult.BlockedMutation -> {
      clearTapHistory()
      cancelPendingPresentation(context = context)
      return true
    }
    EditorInteractiveTapResult.None -> Unit
  }
  if (!context.effects.requestEditing(editor)) {
    clearTapHistory()
    return false
  }
  val dispatched =
    dispatchEditableTap(
      position = position,
      clickCount = 1,
      inputModifiers = inputModifiers.copy(shift = false),
      shouldOpenContextMenu = shouldOpenContextMenu,
      doubleTapDrag = doubleTapDrag,
      context = context,
      preserveExistingSelectionOnSingleTap = false,
      beforeLaunch = {},
    )
  if (dispatched) {
    context.effects.requestSoftwareKeyboard()
  }
  return dispatched
}

private fun EditorTapGesture.dispatchEditableTap(
  position: Offset,
  clickCount: Int,
  inputModifiers: InputModifiers,
  shouldOpenContextMenu: Boolean,
  doubleTapDrag: EditorDoubleTapDragSession,
  context: EditorGestureContext,
  preserveExistingSelectionOnSingleTap: Boolean = true,
  beforeLaunch: () -> Unit,
): Boolean {
  val point =
    context.geometry.resolvePoint(positionInNode = position)
      ?: run {
        clearTapHistory()
        return false
      }
  if (context.mode.isViewportZooming || point.page < 0) {
    clearTapHistory()
    return false
  }
  val editor = context.editor
  when (
    context.semantics.interactiveHit.handleTap(
      editor = editor,
      point = point,
      editing = context.editing,
      readOnly = context.readOnly,
    )
  ) {
    EditorInteractiveTapResult.HandledViewAction,
    EditorInteractiveTapResult.BlockedMutation -> {
      clearTapHistory()
      cancelPendingPresentation(context = context)
      return true
    }
    EditorInteractiveTapResult.None -> Unit
  }
  val generation =
    beginPendingPresentation(
      editor = editor,
      expectedEditing = true,
      tapCount = clickCount,
      showReadingHint = false,
      context = context,
    )
  val wasFocused = context.isFocused
  context.effects.requestFocus(editor)
  if (wasFocused) {
    context.effects.requestSoftwareKeyboard()
  }
  val hitExistingSelectionAtTap =
    editor.selectionHitTest(page = point.page, x = point.x, y = point.y)
  if (
    clickCount == 1 &&
      preserveExistingSelectionOnSingleTap &&
      context.platform != Platform.Android &&
      hitExistingSelectionAtTap
  ) {
    markPendingSelectionPublished(
      generation = generation,
      snapshot = editor.state,
      showContextMenu = shouldOpenContextMenu,
      context = context,
    )
    return false
  }
  val previousCursor = editor.cursor
  beforeLaunch()
  var candidateSnapshot: EditorState? = null
  var candidateShowsContextMenu = false
  val beforeCommit: (EditorState) -> Unit = { snapshot ->
    candidateSnapshot = snapshot
    if (clickCount == 1) {
      when {
        !snapshot.selection.isCollapsed() -> {
          if (!hitExistingSelectionAtTap) {
            candidateShowsContextMenu = true
            context.effects.requestCurrentSelectionHead(version = snapshot.version)
          }
        }
        isSameCursorTap(previousCursor, snapshot) -> {
          candidateShowsContextMenu = wasFocused
        }
        else -> {
          if (snapshot.cursor != null) {
            context.effects.requestCurrentSelectionHead(version = snapshot.version)
          }
        }
      }
    } else {
      candidateShowsContextMenu = !snapshot.selection.isCollapsed()
    }
  }
  val tap = this
  val afterDispatch: (Boolean) -> Unit = { dispatched ->
    val snapshot = candidateSnapshot
    if (dispatched && snapshot != null) {
      markPendingSelectionPublished(
        generation = generation,
        snapshot = snapshot,
        showContextMenu = shouldOpenContextMenu && candidateShowsContextMenu,
        context = context,
      )
      if (clickCount == 2) {
        doubleTapDrag.onWordSelectionCommitted(tap = tap, context = context)
      }
    } else {
      cancelPendingPresentation(context = context)
    }
  }
  when {
    clickCount <= 1 && inputModifiers.shift ->
      context.semantics.pointSelection.launchSelectionExtension(
        editor = editor,
        point = point,
        beforeCommit = beforeCommit,
        afterDispatch = afterDispatch,
      )
    clickCount <= 1 ->
      context.semantics.pointSelection.launchCursorMove(
        editor = editor,
        point = point,
        beforeCommit = beforeCommit,
        afterDispatch = afterDispatch,
      )
    clickCount == 2 ->
      context.semantics.pointSelection.launchUnitSelection(
        editor = editor,
        point = point,
        unit = SelectionPointUnit.Word,
        beforeCommit = beforeCommit,
        afterDispatch = afterDispatch,
      )
    else ->
      context.semantics.pointSelection.launchUnitSelection(
        editor = editor,
        point = point,
        unit = SelectionPointUnit.Paragraph,
        beforeCommit = beforeCommit,
        afterDispatch = afterDispatch,
      )
  }
  return true
}

private fun isSameCursorTap(previousCursor: CursorMetrics?, nextState: EditorState): Boolean {
  val nextCursor = nextState.cursor ?: return false
  if (
    nextState.selection.isCollapsed() &&
      previousCursor != null &&
      nextCursor.isSamePosition(previousCursor)
  ) {
    return true
  }
  return false
}

private fun CursorMetrics.isSamePosition(other: CursorMetrics): Boolean =
  pageIdx == other.pageIdx &&
    caret.x == other.caret.x &&
    caret.y == other.caret.y &&
    line.y == other.line.y
