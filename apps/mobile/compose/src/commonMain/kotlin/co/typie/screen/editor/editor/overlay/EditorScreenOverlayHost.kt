package co.typie.screen.editor.editor.overlay

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.unit.dp
import co.typie.editor.scroll.EditorScrollMode
import co.typie.editor.scroll.EditorScrollPolicy
import co.typie.editor.scroll.EditorVisibleArea

@Composable
internal fun EditorScreenOverlayHost(
  visibleArea: EditorVisibleArea,
  scrollPolicy: EditorScrollPolicy,
  modifier: Modifier = Modifier,
) {
  Box(modifier = modifier.fillMaxSize()) {
    DebugViewportLine(y = visibleArea.visibleViewportTop, color = Color(0xFF00C853))
    DebugViewportLine(y = visibleArea.visibleViewportBottom, color = Color(0xFF00C853))

    when (scrollPolicy.mode) {
      EditorScrollMode.KeepCursorVisible -> {
        DebugViewportLine(y = scrollPolicy.keepVisibleRange.top, color = Color(0xFFFFAB00))
        DebugViewportLine(y = scrollPolicy.keepVisibleRange.bottom, color = Color(0xFFFFAB00))
      }

      EditorScrollMode.Typewriter -> {
        scrollPolicy.typewriterTargetTop?.let {
          DebugViewportLine(y = it, color = Color(0xFFFFAB00))
        }
        scrollPolicy.typewriterTargetBottom
          ?.takeIf { scrollPolicy.typewriterCursorHeight > 0f }
          ?.let { DebugViewportLine(y = it, color = Color(0xFFFFAB00)) }
      }
    }

    // TODO(editor-parity): selection handle, magnifier, scrollbar, extension area 기준
    // affordance 같은 screen/body 오버레이를 채워 넣어야 한다.
  }
}

@Composable
private fun DebugViewportLine(y: Float, color: Color) {
  Box(
    modifier =
      Modifier.fillMaxWidth()
        .height(2.dp)
        .graphicsLayer { translationY = y.dp.toPx() }
        .background(color.copy(alpha = 0.9f))
  )
}
