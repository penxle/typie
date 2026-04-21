package co.typie.shell

import kotlin.math.abs

fun stableIndicatorDirection(
  previousDirection: Float,
  from: Float,
  to: Float,
  minDelta: Float = 1f,
): Float {
  val delta = to - from
  return when {
    delta > minDelta -> 1f
    delta < -minDelta -> -1f
    else -> previousDirection
  }
}

fun bottomBarStretchIntensityForDelta(delta: Float, fullStretchDelta: Float = 18f): Float {
  require(fullStretchDelta > 0f) { "fullStretchDelta must be greater than 0" }
  return (abs(delta) / fullStretchDelta).coerceIn(0f, 1f)
}

data class BottomBarIndicatorDeformerInput(
  val centerX: Float,
  val baseWidth: Float,
  val direction: Float,
  val stretchIntensity: Float,
  val trackStartX: Float,
  val trackEndX: Float,
)

data class BottomBarIndicatorShape(val leftX: Float, val rightX: Float) {
  val width: Float
    get() = rightX - leftX

  val centerX: Float
    get() = (leftX + rightX) / 2f
}

fun interface BottomBarIndicatorDeformer {
  fun deform(input: BottomBarIndicatorDeformerInput): BottomBarIndicatorShape
}

class DirectionalStretchBottomBarIndicatorDeformer(
  private val maxStretchFactor: Float = 0.16f,
  private val trailingStretchFraction: Float = 0.35f,
) : BottomBarIndicatorDeformer {
  override fun deform(input: BottomBarIndicatorDeformerInput): BottomBarIndicatorShape {
    val centeredShape =
      centeredShape(
        centerX = input.centerX,
        width = input.baseWidth,
        trackStartX = input.trackStartX,
        trackEndX = input.trackEndX,
      )

    if (input.direction == 0f || input.baseWidth <= 0f) {
      return centeredShape
    }

    val stretchProgress = input.stretchIntensity.coerceIn(0f, 1f)
    val leadingStretch = input.baseWidth * maxStretchFactor * stretchProgress
    val trailingStretch = leadingStretch * trailingStretchFraction

    var leftX = centeredShape.leftX
    var rightX = centeredShape.rightX

    if (input.direction > 0f) {
      leftX -= trailingStretch
      rightX += leadingStretch
    } else {
      leftX -= leadingStretch
      rightX += trailingStretch
    }

    val trackWidth = (input.trackEndX - input.trackStartX).coerceAtLeast(0f)
    if (rightX - leftX >= trackWidth) {
      return BottomBarIndicatorShape(leftX = input.trackStartX, rightX = input.trackEndX)
    }

    if (leftX < input.trackStartX) {
      val shift = input.trackStartX - leftX
      leftX += shift
      rightX += shift
    }

    if (rightX > input.trackEndX) {
      val shift = rightX - input.trackEndX
      leftX -= shift
      rightX -= shift
    }

    return BottomBarIndicatorShape(
      leftX = leftX.coerceAtLeast(input.trackStartX),
      rightX = rightX.coerceAtMost(input.trackEndX),
    )
  }

  private fun centeredShape(
    centerX: Float,
    width: Float,
    trackStartX: Float,
    trackEndX: Float,
  ): BottomBarIndicatorShape {
    val halfWidth = width / 2f
    val clampedCenterX =
      when {
        trackEndX - trackStartX <= width -> (trackStartX + trackEndX) / 2f
        else -> centerX.coerceIn(trackStartX + halfWidth, trackEndX - halfWidth)
      }

    return BottomBarIndicatorShape(
      leftX = clampedCenterX - halfWidth,
      rightX = clampedCenterX + halfWidth,
    )
  }
}
