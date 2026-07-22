package co.typie.editor.interaction.gestures

import androidx.compose.ui.geometry.Offset
import co.typie.editor.PagePoint
import co.typie.editor.ext.isCollapsed
import co.typie.editor.interaction.EditorGestureContext
import co.typie.editor.interaction.semantics.EditorLongPressSemanticIntent
import co.typie.editor.interaction.sessions.EditorLongPressSession
import co.typie.platform.Platform

internal const val EditorLongPressDispatchDelayMillis = 500L

internal class EditorLongPressGesture {
  private val session = EditorLongPressSession()
  private var pendingPointerId: Long? = null

  private var semanticIntentAtPointerDown: EditorLongPressSemanticIntent? = null

  val capturedSemanticIntent: EditorLongPressSemanticIntent?
    get() = semanticIntentAtPointerDown

  val active: Boolean
    get() = session.active

  val isWordSelection: Boolean
    get() = session.isWordSelection

  fun prepare(pointerId: Long) {
    pendingPointerId = pointerId
  }

  fun captureSemanticIntentAtPointerDown(intent: EditorLongPressSemanticIntent) {
    semanticIntentAtPointerDown = intent
  }

  fun canStart(pointerId: Long): Boolean = pendingPointerId == pointerId && !session.active

  fun isActivePointer(pointerId: Long): Boolean = session.isActivePointer(pointerId)

  fun cancelPending(pointerId: Long? = null): Boolean {
    if (pointerId == null || pendingPointerId == pointerId) {
      pendingPointerId = null
      semanticIntentAtPointerDown = null
      return true
    }
    return false
  }

  fun end() {
    pendingPointerId = null
    session.end()
    semanticIntentAtPointerDown = null
  }

  fun reset() {
    end()
  }

  fun startSession(
    pointerId: Long,
    position: Offset,
    point: PagePoint,
    semanticIntent: EditorLongPressSemanticIntent,
    context: EditorGestureContext,
  ): Boolean {
    pendingPointerId = null
    semanticIntentAtPointerDown = null
    return session.start(
      pointerId = pointerId,
      position = position,
      point = point,
      semanticIntent = semanticIntent,
      context = context,
    )
  }

  fun updateSession(position: Offset, context: EditorGestureContext): Boolean =
    session.update(position = position, context = context)

  fun finishSession(context: EditorGestureContext): Boolean = session.finish(context = context)
}

internal fun EditorLongPressGesture.captureSemanticIntentAtPointerDown(
  position: Offset,
  context: EditorGestureContext,
) {
  val point = context.geometry.resolvePoint(positionInNode = position) ?: return
  captureSemanticIntentAtPointerDown(
    context.semantics.longPress.resolveIntent(
      editor = context.editor,
      point = point,
      platform = context.platform,
    )
  )
}

internal fun EditorLongPressGesture.start(
  pointerId: Long,
  position: Offset,
  context: EditorGestureContext,
): Boolean {
  if (!canStart(pointerId)) {
    return false
  }
  val point = resolveAdmission(position = position, context = context) ?: return false
  val semanticIntent =
    capturedSemanticIntent
      ?: context.semantics.longPress.resolveIntent(
        editor = context.editor,
        point = point,
        platform = context.platform,
      )
  return startSession(
    pointerId = pointerId,
    position = position,
    point = point,
    semanticIntent = semanticIntent,
    context = context,
  )
}

internal fun EditorLongPressGesture.update(
  position: Offset,
  context: EditorGestureContext,
): Boolean = updateSession(position = position, context = context)

internal fun EditorLongPressGesture.finish(context: EditorGestureContext): Boolean =
  finishSession(context = context)

private fun EditorLongPressGesture.resolveAdmission(
  position: Offset,
  context: EditorGestureContext,
): PagePoint? {
  val point = context.geometry.resolvePoint(positionInNode = position)
  if (point == null || point.page < 0) {
    cancelPending()
    return null
  }

  val editor = context.editor
  if (editor.selectionHitTest(page = point.page, x = point.x, y = point.y)) {
    if (context.platform != Platform.Android || !editor.selection.isCollapsed()) {
      cancelPending()
      return null
    }
  }
  return point
}
