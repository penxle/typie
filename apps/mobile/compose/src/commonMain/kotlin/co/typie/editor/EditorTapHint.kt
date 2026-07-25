package co.typie.editor

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.ui.component.Text
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import dev.chrisbanes.haze.HazeState
import dev.chrisbanes.haze.blur.blurEffect
import dev.chrisbanes.haze.hazeEffect
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.collectLatest

internal const val EditorTapHintVisibleMillis = 850
internal const val EditorTapHintFadeMillis = 180
internal const val EditorTapHintTestTag = "editor-tap-hint"

@Composable
internal fun EditorTapHint(
  events: Flow<Unit>,
  text: String,
  hazeState: HazeState,
  modifier: Modifier = Modifier,
) {
  val colors = AppTheme.colors
  val shape = AppShapes.rounded(AppShapes.md)
  val alpha = remember { Animatable(0f) }

  LaunchedEffect(events) {
    events.collectLatest {
      alpha.animateTo(1f, tween(EditorTapHintFadeMillis))
      delay(EditorTapHintVisibleMillis.toLong())
      alpha.animateTo(0f, tween(EditorTapHintFadeMillis))
    }
  }

  if (alpha.value > 0f) {
    Box(
      modifier =
        modifier
          .graphicsLayer { this.alpha = alpha.value }
          .clip(shape)
          .hazeEffect(hazeState) {
            blurEffect {
              backgroundColor = colors.surfaceInset
              blurRadius = 6.dp
            }
          }
          .background(colors.surfaceInset.copy(alpha = 0.36f), shape)
          .semantics { liveRegion = LiveRegionMode.Polite }
          .testTag(EditorTapHintTestTag)
          .padding(horizontal = 14.dp, vertical = 8.dp)
    ) {
      Text(text = text, style = AppTheme.typography.body, color = colors.textMuted)
    }
  }
}

@Composable
internal fun BoxScope.EditorTapHintOverlay(
  events: Flow<Unit>,
  text: String,
  hazeState: HazeState,
  visibleArea: EditorVisibleArea,
) {
  Box(
    modifier =
      Modifier.fillMaxSize()
        .padding(top = visibleArea.visibleViewportTop.dp, bottom = visibleArea.bottomOcclusion.dp),
    contentAlignment = Alignment.Center,
  ) {
    EditorTapHint(events = events, text = text, hazeState = hazeState)
  }
}
