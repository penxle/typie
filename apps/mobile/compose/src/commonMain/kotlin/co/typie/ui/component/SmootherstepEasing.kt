package co.typie.ui.component

import androidx.compose.animation.core.Easing

val SmootherstepEasing: Easing = Easing(::smootherstep)

fun smootherstep(t: Float): Float {
  val x = t.coerceIn(0f, 1f)
  return x * x * x * (x * (x * 6f - 15f) + 10f)
}
