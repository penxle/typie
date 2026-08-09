package co.typie.editor.viewport

import kotlin.math.abs
import kotlin.math.exp
import kotlin.math.max
import kotlin.math.sign
import kotlin.math.sqrt

internal class EditorSmoothScrollMotion
private constructor(
  private var position: Double,
  private var target: Double,
  viewportHeight: Double,
) {
  data class Snapshot(val position: Double, val velocity: Double, val target: Double)

  private var velocity = 0.0
  private var omega = responseRate(position, target, viewportHeight)
  var finished = false
    private set

  init {
    finishIfArrived()
  }

  fun advance(deltaSeconds: Double): Snapshot {
    if (finished || !deltaSeconds.isFinite() || deltaSeconds <= 0.0) {
      return snapshot()
    }
    val previousRemaining = target - position
    val error = position - target
    val b = velocity + omega * error
    val decay = exp(-omega * deltaSeconds)
    position = target + (error + b * deltaSeconds) * decay
    velocity = (velocity - omega * b * deltaSeconds) * decay
    if (previousRemaining * (target - position) < 0.0) {
      finish()
    } else {
      finishIfArrived()
    }
    return snapshot()
  }

  fun retarget(target: Double, viewportHeight: Double) {
    if (!target.isFinite()) return
    this.target = target
    if (abs(target - position) <= PositionThreshold) {
      finish()
      return
    }
    finished = false
    val remaining = target - position
    val towardVelocity = velocity * remaining.sign
    omega =
      max(responseRate(position, target, viewportHeight), max(0.0, towardVelocity) / abs(remaining))
  }

  fun translate(delta: Double) {
    if (!delta.isFinite()) return
    position += delta
    target += delta
  }

  fun synchronizeBounds(actualPosition: Double, maximumScroll: Double, viewportHeight: Double) {
    if (!actualPosition.isFinite() || !maximumScroll.isFinite()) return
    val maximum = max(0.0, maximumScroll)
    position = actualPosition.coerceIn(0.0, maximum)
    target = target.coerceIn(0.0, maximum)
    if ((position <= 0.0 && velocity < 0.0) || (position >= maximum && velocity > 0.0)) {
      velocity = 0.0
    }
    retarget(target, viewportHeight)
  }

  fun cancel() {
    target = position
    finish()
  }

  fun snapshot(): Snapshot = Snapshot(position = position, velocity = velocity, target = target)

  private fun finishIfArrived() {
    if (abs(target - position) <= PositionThreshold && abs(velocity) <= VelocityThreshold) {
      finish()
    }
  }

  private fun finish() {
    position = target
    velocity = 0.0
    finished = true
  }

  companion object {
    private const val MinSettleSeconds = 0.18
    private const val DistanceSettleSeconds = 0.11
    private const val MaxSettleSeconds = 0.65
    private const val OnePercentResponse = 6.64
    private const val PositionThreshold = 0.5
    private const val VelocityThreshold = 5.0

    fun start(position: Double, target: Double, viewportHeight: Double): EditorSmoothScrollMotion =
      EditorSmoothScrollMotion(position, target, viewportHeight)

    private fun responseRate(position: Double, target: Double, viewportHeight: Double): Double {
      val distance = abs(target - position) / max(1.0, viewportHeight)
      val settleSeconds =
        (MinSettleSeconds + DistanceSettleSeconds * sqrt(distance)).coerceAtMost(MaxSettleSeconds)
      return OnePercentResponse / settleSeconds
    }
  }
}
