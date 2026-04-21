package co.typie.ui.theme

import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

@Immutable
data class AppShadowLayer(
  val offsetY: Dp,
  val blur: Dp,
  val spread: Dp = 0.dp,
  val color: Color,
  val offsetX: Dp = 0.dp,
)

@Immutable
data class AppShadow(val layers: List<AppShadowLayer>) {
  companion object {
    val None = AppShadow(emptyList())
  }
}

@Immutable
data class AppShadows(val sm: AppShadow, val md: AppShadow, val lg: AppShadow, val xl: AppShadow)

internal val LightShadowBase = Color(0xFF18160F)
internal val DarkShadowBase = Color(0xFF000000)

val LightAppShadows =
  AppShadows(
    sm =
      AppShadow(
        listOf(
          AppShadowLayer(offsetY = 0.dp, blur = 2.dp, color = LightShadowBase.copy(alpha = 0.04f)),
          AppShadowLayer(offsetY = 1.dp, blur = 4.dp, color = LightShadowBase.copy(alpha = 0.03f)),
        )
      ),
    md =
      AppShadow(
        listOf(
          AppShadowLayer(offsetY = 0.dp, blur = 3.dp, color = LightShadowBase.copy(alpha = 0.04f)),
          AppShadowLayer(offsetY = 2.dp, blur = 8.dp, color = LightShadowBase.copy(alpha = 0.03f)),
        )
      ),
    lg =
      AppShadow(
        listOf(
          AppShadowLayer(offsetY = 0.dp, blur = 4.dp, color = LightShadowBase.copy(alpha = 0.04f)),
          AppShadowLayer(offsetY = 4.dp, blur = 16.dp, color = LightShadowBase.copy(alpha = 0.03f)),
        )
      ),
    xl =
      AppShadow(
        listOf(
          AppShadowLayer(offsetY = 0.dp, blur = 6.dp, color = LightShadowBase.copy(alpha = 0.04f)),
          AppShadowLayer(offsetY = 8.dp, blur = 32.dp, color = LightShadowBase.copy(alpha = 0.03f)),
        )
      ),
  )

val DarkAppShadows =
  AppShadows(
    sm =
      AppShadow(
        listOf(
          AppShadowLayer(offsetY = 0.dp, blur = 2.dp, color = DarkShadowBase.copy(alpha = 0.06f)),
          AppShadowLayer(offsetY = 1.dp, blur = 4.dp, color = DarkShadowBase.copy(alpha = 0.05f)),
        )
      ),
    md =
      AppShadow(
        listOf(
          AppShadowLayer(offsetY = 0.dp, blur = 3.dp, color = DarkShadowBase.copy(alpha = 0.06f)),
          AppShadowLayer(offsetY = 2.dp, blur = 8.dp, color = DarkShadowBase.copy(alpha = 0.05f)),
        )
      ),
    lg =
      AppShadow(
        listOf(
          AppShadowLayer(offsetY = 0.dp, blur = 4.dp, color = DarkShadowBase.copy(alpha = 0.06f)),
          AppShadowLayer(offsetY = 4.dp, blur = 16.dp, color = DarkShadowBase.copy(alpha = 0.05f)),
        )
      ),
    xl =
      AppShadow(
        listOf(
          AppShadowLayer(offsetY = 0.dp, blur = 6.dp, color = DarkShadowBase.copy(alpha = 0.06f)),
          AppShadowLayer(offsetY = 8.dp, blur = 32.dp, color = DarkShadowBase.copy(alpha = 0.05f)),
        )
      ),
  )

val LocalAppShadows = staticCompositionLocalOf { LightAppShadows }

private val DefaultAlpha: () -> Float = { 1f }

fun Modifier.shadow(shadow: AppShadow, shape: Shape, alpha: () -> Float = DefaultAlpha): Modifier {
  if (shadow.layers.isEmpty()) return this
  return shadow.layers.fold(this) { acc, layer ->
    acc.dropShadow(shape) {
      color = layer.color
      offset = Offset(layer.offsetX.toPx(), layer.offsetY.toPx())
      radius = layer.blur.toPx()
      spread = layer.spread.toPx()
      this.alpha = alpha()
    }
  }
}
