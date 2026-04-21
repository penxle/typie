package co.typie.ui.component.topbar

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.LocalInteractionSource
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppTheme

@Composable
fun TopBarButton(
  icon: IconData,
  onClick: (suspend () -> Unit)? = null,
  modifier: Modifier = Modifier,
) {
  val inheritedInteractionSource = LocalInteractionSource.current
  if (inheritedInteractionSource != null) {
    TopBarButtonContent(icon = icon, onClick = onClick, modifier = modifier)
  } else {
    InteractionScope { TopBarButtonContent(icon = icon, onClick = onClick, modifier = modifier) }
  }
}

@Composable
private fun TopBarButtonContent(
  icon: IconData,
  onClick: (suspend () -> Unit)?,
  modifier: Modifier,
) {
  val bg = TopBarDefaults.controlBackgroundColor()
  val borderColor = TopBarDefaults.controlBorderColor()
  val shadowMod = TopBarDefaults.controlShadowModifier(TopBarDefaults.ButtonShape)

  Box(
    contentAlignment = Alignment.Center,
    modifier =
      modifier
        .size(TopBarDefaults.ButtonSize)
        .pressScale(1.05f)
        .then(shadowMod)
        .background(bg, TopBarDefaults.ButtonShape)
        .border(1.dp, borderColor, TopBarDefaults.ButtonShape)
        .then(if (onClick != null) Modifier.clickable(onClick) else Modifier),
  ) {
    Icon(
      icon = icon,
      modifier = Modifier.size(TopBarDefaults.ButtonIconSize),
      tint = AppTheme.colors.textDefault,
    )
  }
}
