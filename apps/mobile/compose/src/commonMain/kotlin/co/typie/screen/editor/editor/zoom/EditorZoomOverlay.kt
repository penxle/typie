package co.typie.screen.editor.editor.zoom

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import co.typie.editor.LocalEditorZoomController
import co.typie.editor.zoomDiffers
import co.typie.ui.component.Text
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.delay

private const val ZoomOverlayVisibleMs = 1000L
private const val ZoomOverlayFadeMs = 180
private val ZoomOverlayWidth = 56.dp
private val ZoomOverlayShape = AppShapes.rounded(8.dp)

@Composable
internal fun EditorZoomOverlay(modifier: Modifier = Modifier) {
  val zoomController = LocalEditorZoomController.current
  val isPaginated = zoomController.isPaginated
  val displayZoom = zoomController.displayZoom

  var visible by remember { mutableStateOf(false) }
  var lastZoom by remember { mutableStateOf<Float?>(null) }
  var wasPaginated by remember { mutableStateOf(false) }
  var showRequest by remember { mutableIntStateOf(0) }

  LaunchedEffect(isPaginated, displayZoom) {
    val enteredPaginated = isPaginated && !wasPaginated
    wasPaginated = isPaginated
    val previousZoom = lastZoom
    lastZoom = displayZoom
    val shouldShow = previousZoom == null || zoomDiffers(previousZoom, displayZoom)

    if (isPaginated && (enteredPaginated || shouldShow)) {
      showRequest += 1
    } else if (!isPaginated) {
      visible = false
    }
  }

  LaunchedEffect(showRequest) {
    if (showRequest <= 0) {
      return@LaunchedEffect
    }

    visible = true
    delay(ZoomOverlayVisibleMs)
    visible = false
  }

  if (!isPaginated) {
    return
  }

  val alpha by
    animateFloatAsState(
      targetValue = if (visible) 1f else 0f,
      animationSpec = tween(durationMillis = ZoomOverlayFadeMs),
      label = "editor-zoom-overlay-alpha",
    )
  val zoomPercent = (displayZoom * 100f).toInt()

  Box(modifier = modifier.graphicsLayer { this.alpha = alpha }) {
    Box(
      modifier =
        Modifier.clip(ZoomOverlayShape)
          .border(1.dp, AppTheme.colors.borderEmphasis, ZoomOverlayShape)
          .background(AppTheme.colors.surfaceInset.copy(alpha = 0.95f), ZoomOverlayShape)
          .width(ZoomOverlayWidth)
          .padding(vertical = 8.dp),
      contentAlignment = Alignment.Center,
    ) {
      Text(
        text = "$zoomPercent%",
        style = AppTheme.typography.caption.copy(fontWeight = FontWeight.W500),
        color = AppTheme.colors.textDefault,
        textAlign = TextAlign.Center,
      )
    }
  }
}
