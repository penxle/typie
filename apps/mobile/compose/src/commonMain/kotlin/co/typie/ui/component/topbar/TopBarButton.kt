package co.typie.ui.component.topbar

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.LocalInteractionSource
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.shadow

@Composable
fun TopBarButton(
  icon: IconData,
  onClick: (suspend () -> Unit)? = null,
  modifier: Modifier = Modifier,
  backgroundColor: Color = TopBarDefaults.controlBackgroundColor(),
  contentColor: Color = AppTheme.colors.textDefault,
) {
  val inheritedInteractionSource = LocalInteractionSource.current
  if (inheritedInteractionSource != null) {
    TopBarButtonContent(
      icon = icon,
      onClick = onClick,
      modifier = modifier,
      backgroundColor = backgroundColor,
      contentColor = contentColor,
    )
  } else {
    InteractionScope {
      TopBarButtonContent(
        icon = icon,
        onClick = onClick,
        modifier = modifier,
        backgroundColor = backgroundColor,
        contentColor = contentColor,
      )
    }
  }
}

@Composable
private fun TopBarButtonContent(
  icon: IconData,
  onClick: (suspend () -> Unit)?,
  modifier: Modifier,
  backgroundColor: Color,
  contentColor: Color,
) {
  val borderColor = TopBarDefaults.controlBorderColor()

  Box(
    contentAlignment = Alignment.Center,
    modifier =
      modifier
        .size(TopBarDefaults.ButtonSize)
        .shadow(AppTheme.shadows.sm, TopBarDefaults.ButtonShape)
        .pressScale(TopBarButtonPressedScale)
        .background(backgroundColor, TopBarDefaults.ButtonShape)
        .border(1.dp, borderColor, TopBarDefaults.ButtonShape)
        .then(if (onClick != null) Modifier.clickable(onClick) else Modifier),
  ) {
    Icon(icon = icon, modifier = Modifier.size(TopBarDefaults.ButtonIconSize), tint = contentColor)
  }
}

private const val TopBarButtonPressedScale = 1.1f
