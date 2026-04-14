package co.typie.ui.component.sheet

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
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
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppTheme

object SheetBarDefaults {
  val SlotWidth: Dp = 44.dp
  val ButtonSize: Dp = SlotWidth
  val ButtonIconSize: Dp = 18.dp
  val ButtonShape: Shape = CircleShape

  @Composable fun controlBackgroundColor(): Color = AppTheme.colors.surfaceRaised

  @Composable fun controlBorderColor(): Color = AppTheme.colors.borderStrong

  @Composable
  fun controlShadowModifier(shape: Shape = ButtonShape): Modifier =
    Modifier.shadow(
      elevation = 4.dp,
      shape = shape,
      ambientColor = AppTheme.colors.shadowAmbient,
      spotColor = AppTheme.colors.shadow,
    )
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
  val resolvedTint = tint ?: AppTheme.colors.textPrimary
  val shadowModifier = SheetBarDefaults.controlShadowModifier(SheetBarDefaults.ButtonShape)

  InteractionScope {
    Box(
      modifier =
        modifier
          .size(SheetBarDefaults.ButtonSize)
          .alpha(alpha)
          .then(shadowModifier)
          .background(resolvedBackground, SheetBarDefaults.ButtonShape)
          .border(1.dp, resolvedBorderColor, SheetBarDefaults.ButtonShape)
          .clickable(enabled = enabled && !loading, onClick = onClick)
          .pressScale(0.94f),
      contentAlignment = Alignment.Center,
    ) {
      if (loading) {
        SheetBarSpinner(color = resolvedTint)
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
  color: Color = AppTheme.colors.textPrimary,
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
          .alpha(alpha)
          .clickable(enabled = enabled && !loading, onClick = onClick)
          .pressScale(0.96f),
      contentAlignment = Alignment.Center,
    ) {
      if (loading) {
        SheetBarSpinner(color = color)
      } else {
        Text(text = text, style = AppTheme.typography.action, color = color)
      }
    }
  }
}

@Composable
private fun SheetBarSpinner(color: Color, modifier: Modifier = Modifier) {
  val transition = rememberInfiniteTransition()
  val rotation by
    transition.animateFloat(
      initialValue = 0f,
      targetValue = 360f,
      animationSpec = infiniteRepeatable(animation = tween(1000, easing = LinearEasing)),
    )

  Canvas(modifier.size(16.dp)) {
    drawArc(
      color = color,
      startAngle = rotation,
      sweepAngle = 220f,
      useCenter = false,
      style =
        androidx.compose.ui.graphics.drawscope.Stroke(
          width = 1.5.dp.toPx(),
          cap = androidx.compose.ui.graphics.StrokeCap.Round,
        ),
    )
  }
}
