package co.typie.ui.component

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable as foundationClickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
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
import co.typie.ext.LocalInteractionSource
import co.typie.ext.pressScale
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme

internal fun resolveSettingSwitchNextChecked(checked: Boolean, indeterminate: Boolean): Boolean {
  return if (indeterminate) true else checked.not()
}

@Composable
fun SettingSwitch(
  checked: Boolean,
  onCheckedChange: (Boolean) -> Unit,
  modifier: Modifier = Modifier,
  enabled: Boolean = true,
  indeterminate: Boolean = false,
) {
  val colors = AppTheme.colors
  val haptic = LocalHapticFeedback.current
  val interactionSource = LocalInteractionSource.current ?: remember { MutableInteractionSource() }
  val trackColor =
    animateColorAsState(
      targetValue =
        if (checked && !indeterminate) colors.brand.copy(alpha = 0.92f) else colors.surfaceTinted,
      animationSpec = tween(durationMillis = 180),
      label = "setting-switch-track",
    )
  val borderColor =
    animateColorAsState(
      targetValue =
        if (checked && !indeterminate) colors.brand.copy(alpha = 0.24f) else colors.borderDefault,
      animationSpec = tween(durationMillis = 180),
      label = "setting-switch-border",
    )
  val thumbOffset =
    animateDpAsState(
      targetValue = if (indeterminate) 9.dp else if (checked) 18.dp else 0.dp,
      animationSpec = tween(durationMillis = 180),
      label = "setting-switch-thumb",
    )

  InteractionScope {
    Box(
      modifier =
        modifier
          .size(width = 46.dp, height = 28.dp)
          .alpha(if (enabled) 1f else 0.5f)
          .clip(RoundedCornerShape(16.dp))
          .then(
            if (enabled) {
              Modifier.foundationClickable(
                interactionSource = interactionSource,
                indication = null,
              ) {
                val next = resolveSettingSwitchNextChecked(checked, indeterminate)
                haptic.performHapticFeedback(
                  if (next) HapticFeedbackType.ToggleOn else HapticFeedbackType.ToggleOff
                )
                onCheckedChange(next)
              }
            } else {
              Modifier
            }
          )
          .pressScale(0.97f)
          .background(trackColor.value, RoundedCornerShape(16.dp))
          .border(1.dp, borderColor.value, RoundedCornerShape(16.dp))
          .padding(PaddingValues(3.dp)),
      contentAlignment = Alignment.CenterStart,
    ) {
      if (indeterminate) {
        Box(
          modifier =
            Modifier.fillMaxHeight()
              .fillMaxWidth(0.5f)
              .clip(RoundedCornerShape(topStart = 13.dp, bottomStart = 13.dp))
              .background(colors.brandSubtle)
              .align(Alignment.CenterStart)
        )
      }

      Box(
        modifier =
          Modifier.offset(x = thumbOffset.value)
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
        contentAlignment = Alignment.Center,
      ) {
        if (indeterminate) {
          Icon(
            icon = co.typie.icons.Lucide.Minus,
            modifier = Modifier.size(12.dp),
            tint = colors.textTertiary,
            strokeWidth = 2.5f,
            contentDescription = null,
          )
        }
      }
    }
  }
}
