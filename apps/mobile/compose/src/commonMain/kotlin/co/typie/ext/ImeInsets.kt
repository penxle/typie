package co.typie.ext

import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.LayoutDirection

@Composable internal expect fun rememberTrustedImeInsets(): WindowInsets

@Composable
internal fun rememberTrustedImeBottomInset(): Dp {
  val density = LocalDensity.current
  return with(density) { rememberTrustedImeInsets().getBottom(this).toDp() }
}

internal fun trustedImeInsets(
  rawImeInsets: WindowInsets,
  settledBottom: (Density) -> Int?,
  presentationBottom: (Density) -> Int? = { null },
): WindowInsets =
  object : WindowInsets {
    override fun getLeft(density: Density, layoutDirection: LayoutDirection) =
      rawImeInsets.getLeft(density, layoutDirection)

    override fun getTop(density: Density) = rawImeInsets.getTop(density)

    override fun getRight(density: Density, layoutDirection: LayoutDirection) =
      rawImeInsets.getRight(density, layoutDirection)

    override fun getBottom(density: Density): Int {
      val rawBottom = rawImeInsets.getBottom(density)
      val settled = settledBottom(density)
      val presentation = presentationBottom(density)
      return presentation ?: if (settled != null && rawBottom > settled) settled else rawBottom
    }
  }

internal fun trustedImeBottomInset(rawImeBottom: Dp, settledImeBottom: Dp?): Dp =
  if (settledImeBottom != null && rawImeBottom > settledImeBottom) {
    settledImeBottom
  } else {
    rawImeBottom
  }
