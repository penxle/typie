package co.typie.ui.component

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.ui.theme.AppTheme

@Composable
fun SettingSwitch(
  checked: Boolean,
  onCheckedChange: suspend (Boolean) -> Unit,
  modifier: Modifier = Modifier,
  enabled: Boolean = true,
) {
  val colors = AppTheme.colors
  val haptic = LocalHapticFeedback.current
  val trackColor = animateColorAsState(
    targetValue = if (checked) colors.brand.copy(alpha = 0.92f) else colors.surfaceTinted,
    animationSpec = tween(durationMillis = 180),
    label = "setting-switch-track",
  )
  val borderColor = animateColorAsState(
    targetValue = if (checked) colors.brand.copy(alpha = 0.24f) else colors.borderDefault,
    animationSpec = tween(durationMillis = 180),
    label = "setting-switch-border",
  )
  val thumbOffset = animateDpAsState(
    targetValue = if (checked) 18.dp else 0.dp,
    animationSpec = tween(durationMillis = 180),
    label = "setting-switch-thumb",
  )

  InteractionScope {
    Box(
      modifier = modifier
        .size(width = 46.dp, height = 28.dp)
        .alpha(if (enabled) 1f else 0.5f)
        .clip(RoundedCornerShape(16.dp))
        .then(
          if (enabled) {
            Modifier.clickable {
              val next = checked.not()
              haptic.performHapticFeedback(
                if (next) HapticFeedbackType.ToggleOn else HapticFeedbackType.ToggleOff,
              )
              onCheckedChange(next)
            }
          } else {
            Modifier
          },
        )
        .pressScale(0.97f)
        .background(trackColor.value, RoundedCornerShape(16.dp))
        .border(1.dp, borderColor.value, RoundedCornerShape(16.dp))
        .padding(PaddingValues(3.dp)),
      contentAlignment = Alignment.CenterStart,
    ) {
      Box(
        modifier = Modifier
          .offset(x = thumbOffset.value)
          .size(22.dp)
          .dropShadow(CircleShape) {
            color = colors.shadowAmbient
            radius = 4f
          }
          .dropShadow(CircleShape) {
            color = colors.shadow
            offset = Offset(0f, 2f)
            radius = 8f
          }
          .background(colors.surfaceRaised, CircleShape),
      )
    }
  }
}
