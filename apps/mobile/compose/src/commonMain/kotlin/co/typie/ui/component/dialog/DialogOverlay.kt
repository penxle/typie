package co.typie.ui.component.dialog

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.unit.dp
import co.typie.ext.clickable
import co.typie.ext.imePadding
import co.typie.navigation.PlatformBackHandler
import co.typie.ui.theme.AppTheme

@Composable
fun DialogOverlay(state: Dialog) {
  val entry = state.current ?: return
  val density = LocalDensity.current
  val focusManager = LocalFocusManager.current
  val offsetPx = with(density) { 20.dp.toPx() }

  var pendingResult by remember(entry) { mutableStateOf<DialogResult<Any?>?>(null) }
  var visible by remember(entry) { mutableStateOf(false) }
  var dialogHasFocus by remember(entry) { mutableStateOf(false) }
  LaunchedEffect(entry) { visible = true }

  val progress by
    animateFloatAsState(
      targetValue = if (visible) 1f else 0f,
      animationSpec = tween(200),
      finishedListener = { value ->
        if (value == 0f) {
          state.resolveCurrentEntry(pendingResult ?: DialogResult.Dismissed)
        }
      },
    )

  val requestDismissal =
    remember(entry, state, focusManager) {
      { result: DialogResult<Any?> ->
        if (entry.acceptsInput) {
          if (dialogHasFocus) {
            focusManager.clearFocus()
          }
          pendingResult = result
          state.stopEntryAcceptingInput(entry)
          visible = false
        }
      }
    }

  val scope =
    remember(entry, requestDismissal) {
      object : DialogScope<Any?> {
        override fun resolve(result: Any?) {
          requestDismissal(DialogResult.Resolved(result))
        }

        override fun dismiss() {
          requestDismissal(DialogResult.Dismissed)
        }
      }
    }

  PlatformBackHandler(enabled = entry.dismissible && entry.acceptsInput) {
    requestDismissal(DialogResult.Dismissed)
  }

  Box(Modifier.fillMaxSize()) {
    Box(
      Modifier.fillMaxSize()
        .graphicsLayer { alpha = progress }
        .background(AppTheme.colors.scrim)
        .then(
          if (entry.dismissible) Modifier.clickable { requestDismissal(DialogResult.Dismissed) }
          else Modifier.pointerInput(Unit) {}
        )
    )

    Box(Modifier.fillMaxSize().imePadding()) {
      Box(
        modifier =
          Modifier.align(Alignment.Center)
            .width(280.dp)
            .onFocusChanged { dialogHasFocus = it.hasFocus }
            .pointerInput(Unit) {}
            .graphicsLayer {
              alpha = progress
              translationY = (1f - progress) * offsetPx
            }
      ) {
        @Suppress("UNCHECKED_CAST") val typedEntry = entry as DialogEntry<Any?>
        context(scope) { typedEntry.content() }
      }
    }
  }
}
