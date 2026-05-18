package co.typie.ui.component.sheet

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.ui.component.Spinner
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.shadow

object SheetBarDefaults {
  val SlotWidth: Dp = 44.dp
  val ButtonSize: Dp = SlotWidth
  val ButtonIconSize: Dp = 18.dp
  val ButtonShape: Shape = AppShapes.circle

  @Composable fun controlBackgroundColor(): Color = AppTheme.colors.surfaceDefault

  @Composable fun controlBorderColor(): Color = AppTheme.colors.borderEmphasis
}

@Composable
fun SheetBar(
  modifier: Modifier = Modifier,
  leading: (@Composable () -> Unit)? = null,
  center: (@Composable () -> Unit)? = null,
  trailing: (@Composable () -> Unit)? = null,
) {
  val leadingInset = if (leading != null) SheetBarDefaults.SlotWidth + 12.dp else 0.dp
  val trailingInset = if (trailing != null) SheetBarDefaults.SlotWidth + 12.dp else 0.dp
  val centerInset = maxOf(leadingInset, trailingInset)

  Box(modifier = modifier.fillMaxWidth().height(SheetBarDefaults.SlotWidth)) {
    if (center != null) {
      Box(
        modifier =
          Modifier.align(Alignment.Center)
            .fillMaxWidth()
            .padding(start = centerInset, end = centerInset),
        contentAlignment = Alignment.Center,
      ) {
        center()
      }
    }

    Row(
      modifier = Modifier.fillMaxWidth(),
      horizontalArrangement = Arrangement.SpaceBetween,
      verticalAlignment = Alignment.CenterVertically,
    ) {
      Box(
        modifier = Modifier.size(SheetBarDefaults.SlotWidth),
        contentAlignment = Alignment.CenterStart,
      ) {
        leading?.invoke()
      }

      Box(
        modifier = Modifier.size(SheetBarDefaults.SlotWidth),
        contentAlignment = Alignment.CenterEnd,
      ) {
        trailing?.invoke()
      }
    }
  }
}

@Composable
fun SheetBarButton(
  icon: IconData,
  onClick: suspend () -> Unit,
  modifier: Modifier = Modifier,
  enabled: Boolean = true,
  loading: Boolean = false,
  backgroundColor: Color? = null,
  borderColor: Color? = null,
  tint: Color? = null,
) {
  val alpha by animateFloatAsState(if (enabled) 1f else 0.4f)
  val resolvedBackground = backgroundColor ?: SheetBarDefaults.controlBackgroundColor()
  val resolvedBorderColor = borderColor ?: SheetBarDefaults.controlBorderColor()
  val resolvedTint = tint ?: AppTheme.colors.textDefault

  InteractionScope {
    Box(
      modifier =
        modifier
          .size(SheetBarDefaults.ButtonSize)
          .graphicsLayer { this.alpha = alpha }
          .shadow(AppTheme.shadows.md, SheetBarDefaults.ButtonShape)
          .border(1.dp, resolvedBorderColor, SheetBarDefaults.ButtonShape)
          .background(resolvedBackground, SheetBarDefaults.ButtonShape)
          .clickable(enabled = enabled && !loading, onClick = onClick)
          .pressScale(0.94f),
      contentAlignment = Alignment.Center,
    ) {
      if (loading) {
        Spinner(color = resolvedTint)
      } else {
        Icon(
          icon = icon,
          modifier = Modifier.size(SheetBarDefaults.ButtonIconSize),
          tint = resolvedTint,
        )
      }
    }
  }
}

@Composable
fun SheetBarTextButton(
  text: String,
  onClick: suspend () -> Unit,
  modifier: Modifier = Modifier,
  enabled: Boolean = true,
  loading: Boolean = false,
  color: Color = AppTheme.colors.textDefault,
) {
  val alpha by animateFloatAsState(if (enabled) 1f else 0.4f)

  InteractionScope {
    Box(
      modifier =
        modifier
          .defaultMinSize(
            minWidth = SheetBarDefaults.SlotWidth,
            minHeight = SheetBarDefaults.SlotWidth,
          )
          .graphicsLayer { this.alpha = alpha }
          .clickable(enabled = enabled && !loading, onClick = onClick)
          .pressScale(0.96f),
      contentAlignment = Alignment.Center,
    ) {
      if (loading) {
        Spinner(color = color)
      } else {
        Text(text = text, style = AppTheme.typography.action, color = color)
      }
    }
  }
}
