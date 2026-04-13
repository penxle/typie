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
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import co.typie.ext.clickable
import co.typie.navigation.PlatformBackHandler
import co.typie.ui.theme.AppTheme

@Composable
fun DialogOverlay(state: Dialog) {
  val entry = state.current ?: return
  val density = LocalDensity.current
  val offsetPx = with(density) { 20.dp.toPx() }

  var pendingResult by remember(entry) { mutableStateOf<DialogResult<Any?>?>(null) }
  var visible by remember(entry) { mutableStateOf(false) }
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

  val scope =
    remember(entry) {
      object : DialogScope<Any?> {
        override fun resolve(result: Any?) {
          pendingResult = DialogResult.Resolved(result)
          visible = false
        }

        override fun dismiss() {
          pendingResult = DialogResult.Dismissed
          visible = false
        }
      }
    }

  PlatformBackHandler(enabled = entry.dismissible) {
    pendingResult = DialogResult.Dismissed
    visible = false
  }

  Box(Modifier.fillMaxSize()) {
    Box(
      Modifier.fillMaxSize()
        .alpha(progress)
        .background(AppTheme.colors.scrim)
        .then(
          if (entry.dismissible)
            Modifier.clickable {
              pendingResult = DialogResult.Dismissed
              visible = false
            }
          else Modifier
        )
    )

    Box(
      modifier =
        Modifier.align(Alignment.Center)
          .width(280.dp)
          .graphicsLayer(alpha = progress, translationY = (1f - progress) * offsetPx)
    ) {
      @Suppress("UNCHECKED_CAST") val typedEntry = entry as DialogEntry<Any?>
      context(scope) { typedEntry.content() }
    }
  }
}
