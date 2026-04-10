@file:kotlin.jvm.JvmName("AndroidModifierExt")

package co.typie.ext

import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.gestures.ScrollableState
import androidx.compose.ui.Modifier
import androidx.compose.ui.composed
import androidx.compose.foundation.horizontalScroll as foundationHorizontalScroll
import androidx.compose.foundation.verticalScroll as foundationVerticalScroll

actual fun Modifier.verticalScroll(state: ScrollState, enabled: Boolean): Modifier = composed {
  val isLocked = LocalScrollGestureLockState.current.isLocked
  foundationVerticalScroll(state, enabled = enabled && !isLocked)
}

actual fun Modifier.horizontalScroll(state: ScrollState, enabled: Boolean): Modifier = composed {
  val isLocked = LocalScrollGestureLockState.current.isLocked
  foundationHorizontalScroll(state, enabled = enabled && !isLocked)
}

internal actual fun Modifier.desktopDragScroll(
  state: ScrollableState,
  orientation: androidx.compose.foundation.gestures.Orientation,
  enabled: Boolean,
): Modifier = this
