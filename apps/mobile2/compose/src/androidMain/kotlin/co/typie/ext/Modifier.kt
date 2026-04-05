@file:kotlin.jvm.JvmName("AndroidModifierExt")

package co.typie.ext

import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.ScrollableState
import androidx.compose.foundation.gestures.scrollable
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

actual fun Modifier.overscroll(): Modifier = this

actual fun Modifier.dragScrollable(state: ScrollableState, orientation: Orientation, enabled: Boolean): Modifier = this
