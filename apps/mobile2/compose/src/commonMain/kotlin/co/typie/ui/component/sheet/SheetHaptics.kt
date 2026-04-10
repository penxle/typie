package co.typie.ui.component.sheet

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalHapticFeedback

enum class SheetHapticEvent {
  Present,
  DetentSnap,
  Dismiss,
}

interface SheetHaptics {
  fun perform(event: SheetHapticEvent)
}

@Composable
fun rememberSheetHaptics(): SheetHaptics {
  val haptic = LocalHapticFeedback.current
  return remember(haptic) {
    object : SheetHaptics {
      override fun perform(event: SheetHapticEvent) {
        val type = when (event) {
          SheetHapticEvent.Present -> HapticFeedbackType.GestureThresholdActivate
          SheetHapticEvent.DetentSnap -> HapticFeedbackType.SegmentTick
          SheetHapticEvent.Dismiss -> HapticFeedbackType.GestureEnd
        }
        haptic.performHapticFeedback(type)
      }
    }
  }
}
