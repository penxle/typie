package co.typie.ui.component.topbar

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.LocalInteractionSource
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.ui.component.tooltip.tooltip
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.input.hoverFeedback
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.shadow

@Composable
fun TopBarButton(
  icon: IconData,
  contentDescription: String,
  onClick: (suspend () -> Unit)? = null,
  modifier: Modifier = Modifier,
  backgroundColor: Color = TopBarDefaults.controlBackgroundColor(),
  contentColor: Color = AppTheme.colors.textDefault,
  shortcut: String? = null,
) {
  val inheritedInteractionSource = LocalInteractionSource.current
  if (inheritedInteractionSource != null) {
    TopBarButtonContent(
      icon = icon,
      contentDescription = contentDescription,
      onClick = onClick,
      modifier = modifier,
      backgroundColor = backgroundColor,
      contentColor = contentColor,
      shortcut = shortcut,
    )
  } else {
    InteractionScope {
      TopBarButtonContent(
        icon = icon,
        contentDescription = contentDescription,
        onClick = onClick,
        modifier = modifier,
        backgroundColor = backgroundColor,
        contentColor = contentColor,
        shortcut = shortcut,
      )
    }
  }
}

@Composable
private fun TopBarButtonContent(
  icon: IconData,
  contentDescription: String,
  onClick: (suspend () -> Unit)?,
  modifier: Modifier,
  backgroundColor: Color,
  contentColor: Color,
  shortcut: String?,
) {
  val borderColor = TopBarDefaults.controlBorderColor()
  val isDefaultBackground = backgroundColor == TopBarDefaults.controlBackgroundColor()

  Box(
    contentAlignment = Alignment.Center,
    modifier =
      modifier
        .size(TopBarDefaults.ButtonSize)
        .semantics { role = Role.Button }
        .tooltip(contentDescription, shortcut)
        .shadow(AppTheme.shadows.sm, TopBarDefaults.ButtonShape)
        .pressScale(TopBarButtonPressedScale)
        .background(backgroundColor, TopBarDefaults.ButtonShape)
        .border(1.dp, borderColor, TopBarDefaults.ButtonShape)
        .hoverFeedback(
          shape = TopBarDefaults.ButtonShape,
          hoverColor =
            if (isDefaultBackground) AppTheme.colors.surfaceHover
            else lerp(backgroundColor, Color.Black, 0.06f),
          activeColor =
            if (isDefaultBackground) AppTheme.colors.surfaceActive
            else lerp(backgroundColor, Color.Black, 0.20f),
        )
        .then(if (onClick != null) Modifier.clickable(onClick = onClick) else Modifier),
  ) {
    Icon(
      icon = icon,
      contentDescription = contentDescription,
      modifier = Modifier.size(TopBarDefaults.ButtonIconSize),
      tint = contentColor,
    )
  }
}

private const val TopBarButtonPressedScale = 1.1f
