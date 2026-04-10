package co.typie.screen.home.home

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.EaseInOutExpo
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.border
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.ext.clickable
import co.typie.ui.theme.AppTheme

object HomeSearchFieldDefaults {
  val Height: Dp = 48.dp
  val HorizontalPadding: Dp = 16.dp
  val CornerRadius: Dp = 12.dp
  val IconSize: Dp = 16.dp
  val IconGap: Dp = 10.dp
  val ClearIconGap: Dp = 8.dp
  val FocusedBorderWidth: Dp = 1.5.dp
  val UnfocusedBorderWidth: Dp = 1.dp

  val Shape = RoundedCornerShape(CornerRadius)
}

internal fun resolveHomeSearchPlaceholder(spaceName: String?): String {
  val trimmedSpaceName = spaceName?.trim().orEmpty()

  return if (trimmedSpaceName.isNotEmpty()) {
    "${trimmedSpaceName}에서 검색..."
  } else {
    "문서 검색..."
  }
}

@Composable
fun HomeSearchFieldFrame(
  modifier: Modifier = Modifier,
  focused: Boolean = false,
  enabled: Boolean = true,
  onClick: (suspend () -> Unit)? = null,
  content: @Composable RowScope.() -> Unit,
) {
  val colorSpec = tween<Color>(220)
  val containerColor by animateColorAsState(
    when {
      !enabled -> AppTheme.colors.surfaceBase
      else -> AppTheme.colors.surfaceDefault
    },
    colorSpec,
  )
  val borderColor by animateColorAsState(
    if (focused) AppTheme.colors.borderStrong else AppTheme.colors.borderSubtle,
    colorSpec,
  )
  val borderWidth by animateDpAsState(
    if (focused) HomeSearchFieldDefaults.FocusedBorderWidth else HomeSearchFieldDefaults.UnfocusedBorderWidth,
    tween(durationMillis = 220, easing = EaseInOutExpo),
  )

  Row(
    verticalAlignment = Alignment.CenterVertically,
    modifier = modifier
      .height(HomeSearchFieldDefaults.Height)
      .border(borderWidth, borderColor, HomeSearchFieldDefaults.Shape)
      .background(containerColor, HomeSearchFieldDefaults.Shape)
      .then(if (onClick != null) Modifier.clickable(onClick = onClick) else Modifier)
      .padding(horizontal = HomeSearchFieldDefaults.HorizontalPadding),
  ) {
    content()
  }
}
