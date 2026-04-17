package co.typie.domain.settings

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
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.LocalInteractionSource
import co.typie.ext.pressScale
import co.typie.icons.Lucide
import co.typie.ui.icon.Icon
import co.typie.ui.skeleton.LocalSkeleton
import co.typie.ui.skeleton.skeletonBone
import co.typie.ui.theme.AppShapes
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
  val isSkeleton = LocalSkeleton.current.enabled

  Box(modifier.size(width = 46.dp, height = 28.dp).skeletonBone(AppShapes.rounded(AppShapes.lg))) {
    if (isSkeleton) return@Box

    val colors = AppTheme.colors
    val haptic = LocalHapticFeedback.current
    val interactionSource =
      LocalInteractionSource.current ?: remember { MutableInteractionSource() }
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
          Modifier.fillMaxSize()
            .graphicsLayer {
              alpha = if (enabled) 1f else 0.5f
              shape = AppShapes.rounded(AppShapes.lg)
              clip = true
            }
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
            .background(trackColor.value, AppShapes.rounded(AppShapes.lg))
            .border(1.dp, borderColor.value, AppShapes.rounded(AppShapes.lg))
            .padding(PaddingValues(3.dp)),
        contentAlignment = Alignment.CenterStart,
      ) {
        if (indeterminate) {
          Box(
            modifier =
              Modifier.fillMaxHeight()
                .fillMaxWidth(0.5f)
                .clip(RoundedCornerShape(topStart = AppShapes.lg, bottomStart = AppShapes.lg))
                .background(colors.brandSubtle)
                .align(Alignment.CenterStart)
          )
        }

        Box(
          modifier =
            Modifier.graphicsLayer { translationX = thumbOffset.value.toPx() }
              .size(22.dp)
              .dropShadow(AppShapes.circle) {
                color = colors.shadowAmbient
                radius = 4f
              }
              .dropShadow(AppShapes.circle) {
                color = colors.shadow
                offset = Offset(0f, 2f)
                radius = 8f
              }
              .background(colors.surfaceRaised, AppShapes.circle),
          contentAlignment = Alignment.Center,
        ) {
          if (indeterminate) {
            Icon(
              icon = Lucide.Minus,
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
}
