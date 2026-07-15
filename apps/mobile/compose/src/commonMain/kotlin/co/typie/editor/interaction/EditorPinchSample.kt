package co.typie.editor.interaction

import androidx.compose.ui.geometry.Offset

internal data class EditorPinchSample(val focalInRootPx: Offset, val distancePx: Float)

internal fun resolveEditorPinchSample(positionsInRoot: List<Offset>): EditorPinchSample? {
  if (positionsInRoot.size != 2) {
    return null
  }
  val first = positionsInRoot[0]
  val second = positionsInRoot[1]
  return EditorPinchSample(
    focalInRootPx = (first + second) / 2f,
    distancePx = (first - second).getDistance(),
  )
}
