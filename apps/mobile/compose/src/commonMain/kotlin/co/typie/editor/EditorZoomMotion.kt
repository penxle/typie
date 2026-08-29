package co.typie.editor

import kotlin.math.abs
import kotlin.math.exp
import kotlin.math.ln

internal object EditorZoomMotionTuning {
  const val ElasticExtentRatio = 1.25
  const val ElasticResistance = 0.55
  const val MaximumMotionSeconds = 0.24
  const val SpringAngularFrequency = 24.0
  const val SettleEpsilon = 0.0005
}

internal data class EditorZoomMotionFrame(val displayZoom: Float, val finished: Boolean)

internal fun elasticEditorDisplayZoom(
  rawZoom: Float,
  bounds: ClosedFloatingPointRange<Float>,
): Float? {
  if (!rawZoom.isValidZoom || !bounds.isValidZoomBounds) return null
  val rawLog = ln(rawZoom.toDouble())
  val minimumLog = ln(bounds.start.toDouble())
  val maximumLog = ln(bounds.endInclusive.toDouble())
  val elasticExtent = ln(EditorZoomMotionTuning.ElasticExtentRatio)
  val displayLog =
    when {
      rawLog > maximumLog -> maximumLog + rubberBandDistance(rawLog - maximumLog, elasticExtent)
      rawLog < minimumLog -> minimumLog - rubberBandDistance(minimumLog - rawLog, elasticExtent)
      else -> rawLog
    }
  return exp(displayLog).toFloat()
}

internal class EditorZoomMotion(displayZoom: Float, bounds: ClosedFloatingPointRange<Float>) {
  private var elapsedSeconds = 0.0
  private val initialLogZoom: Double
  private val targetLogZoom: Double

  private var frame: EditorZoomMotionFrame

  init {
    require(displayZoom.isValidZoom && bounds.isValidZoomBounds && displayZoom !in bounds) {
      "Zoom recovery requires an out-of-bounds display zoom"
    }
    initialLogZoom = ln(displayZoom.toDouble())
    targetLogZoom =
      ln(
        if (displayZoom < bounds.start) bounds.start.toDouble() else bounds.endInclusive.toDouble()
      )
    frame = EditorZoomMotionFrame(displayZoom = displayZoom, finished = false)
  }

  fun advance(deltaSeconds: Double): EditorZoomMotionFrame {
    if (!deltaSeconds.isFinite() || deltaSeconds <= 0.0 || frame.finished) return frame
    elapsedSeconds += deltaSeconds
    val logZoom = springLogZoom(elapsedSeconds)
    val finished =
      elapsedSeconds >= EditorZoomMotionTuning.MaximumMotionSeconds || isSettled(logZoom)
    frame =
      EditorZoomMotionFrame(
        displayZoom = exp(if (finished) targetLogZoom else logZoom).toFloat(),
        finished = finished,
      )
    return frame
  }

  private fun springLogZoom(elapsed: Double): Double {
    val displacement = initialLogZoom - targetLogZoom
    val angularFrequency = EditorZoomMotionTuning.SpringAngularFrequency
    val decay = exp(-angularFrequency * elapsed)
    return targetLogZoom + displacement * (1.0 + angularFrequency * elapsed) * decay
  }

  private fun isSettled(logZoom: Double): Boolean =
    abs(logZoom - targetLogZoom) <= EditorZoomMotionTuning.SettleEpsilon
}

private fun rubberBandDistance(distance: Double, extent: Double): Double =
  (extent * EditorZoomMotionTuning.ElasticResistance * distance) /
    (extent + EditorZoomMotionTuning.ElasticResistance * distance)

private val Float.isValidZoom: Boolean
  get() = isFinite() && this > 0f

private val ClosedFloatingPointRange<Float>.isValidZoomBounds: Boolean
  get() = start.isValidZoom && endInclusive.isValidZoom && endInclusive >= start
