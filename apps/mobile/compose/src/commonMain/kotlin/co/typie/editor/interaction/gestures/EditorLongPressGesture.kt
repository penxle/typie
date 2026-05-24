package co.typie.editor.interaction.gestures

import androidx.compose.ui.geometry.Offset
import co.typie.editor.PagePoint
import co.typie.editor.ext.isCollapsed
import co.typie.editor.interaction.EditorGestureContext
import co.typie.editor.interaction.sessions.EditorLongPressSemanticIntent
import co.typie.editor.interaction.sessions.EditorLongPressSession
import co.typie.platform.Platform

internal const val EditorLongPressDispatchDelayMillis = 500L

internal class EditorLongPressGesture {
  private val session = EditorLongPressSession()
  private var pendingPointerId: Long? = null

  var androidUseCursorModeAtPointerDown: Boolean? = null
    private set

  val active: Boolean
    get() = session.active

  val isWordSelection: Boolean
    get() = session.isWordSelection

  fun prepare(pointerId: Long) {
    pendingPointerId = pointerId
  }

  fun primeAndroidSemanticAtPointerDown(useCursorMode: Boolean) {
    androidUseCursorModeAtPointerDown = useCursorMode
  }

  fun canStart(pointerId: Long): Boolean = pendingPointerId == pointerId && !session.active

  fun isActivePointer(pointerId: Long): Boolean = session.isActivePointer(pointerId)

  fun cancelPending(pointerId: Long? = null): Boolean {
    if (pointerId == null || pendingPointerId == pointerId) {
      pendingPointerId = null
      androidUseCursorModeAtPointerDown = null
      return true
    }
    return false
  }

  fun end() {
    pendingPointerId = null
    session.end()
    androidUseCursorModeAtPointerDown = null
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
    androidUseCursorModeAtPointerDown = null
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

internal fun EditorLongPressGesture.primeModeAtPointerDown(
  position: Offset,
  context: EditorGestureContext,
) {
  if (context.platform != Platform.Android) {
    return
  }
  val point = context.geometry.resolvePoint(positionInNode = position) ?: return
  primeAndroidSemanticAtPointerDown(useCursorMode = shouldUseAndroidCursorMode(point, context))
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
  val semanticIntent = resolveSemanticIntent(point = point, context = context)
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

private fun EditorLongPressGesture.resolveSemanticIntent(
  point: PagePoint,
  context: EditorGestureContext,
): EditorLongPressSemanticIntent {
  if (context.platform != Platform.Android) {
    return EditorLongPressSemanticIntent.CursorMove
  }
  val useCursorMode =
    androidUseCursorModeAtPointerDown ?: shouldUseAndroidCursorMode(point, context)
  return if (useCursorMode) {
    EditorLongPressSemanticIntent.CursorMove
  } else {
    EditorLongPressSemanticIntent.WordSelection
  }
}

private fun shouldUseAndroidCursorMode(point: PagePoint, context: EditorGestureContext): Boolean {
  val editor = context.editor
  if (!editor.selection.isCollapsed()) {
    return false
  }
  return editor.cursorHitTest(page = point.page, x = point.x, y = point.y)
}
