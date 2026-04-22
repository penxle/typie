package co.typie.screen.editor.editor.toolbar

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import co.typie.ext.ime
import co.typie.ui.component.ResponsiveContainerDefaults
import co.typie.ui.component.Text
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

private val ToolbarBottomPadding = 12.dp
private const val ToolbarSurfaceAlpha = 0.4f

@Composable
internal fun EditorToolbarHost(bodyFocused: Boolean, modifier: Modifier = Modifier) {
  val imeBottom = WindowInsets.ime.asPaddingValues().calculateBottomPadding()
  val imeVisible = imeBottom > 0.dp
  val visible = shouldShowEditorToolbar(bodyFocused = bodyFocused, imeVisible = imeVisible)

  AnimatedVisibility(
    visible = visible,
    enter = fadeIn(),
    exit = fadeOut(),
    modifier = modifier.fillMaxWidth(),
  ) {
    Box(
      modifier =
        Modifier.fillMaxWidth()
          .offset { IntOffset(x = 0, y = -imeBottom.roundToPx()) }
          .padding(start = 16.dp, end = 16.dp, bottom = ToolbarBottomPadding),
      contentAlignment = Alignment.BottomCenter,
    ) {
      Box(
        modifier =
          Modifier.widthIn(max = ResponsiveContainerDefaults.MaxWidth)
            .fillMaxWidth()
            .height(48.dp)
            .background(
              AppTheme.colors.surfaceDefault.copy(alpha = ToolbarSurfaceAlpha),
              AppShapes.rounded(AppShapes.md),
            )
            .border(1.dp, AppTheme.colors.borderDefault, AppShapes.rounded(AppShapes.md)),
        contentAlignment = Alignment.Center,
      ) {
        // TODO(editor-parity): 이 placeholder bar를 에디터 selection 상태와 키보드 표시
        // 여부에 반응하는 bottom/floating/secondary toolbar로 교체해야 한다.
        Text(
          text = "편집 도구 준비 중",
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textMuted,
        )
      }
    }
  }
}

internal fun shouldShowEditorToolbar(bodyFocused: Boolean, imeVisible: Boolean): Boolean =
  bodyFocused && imeVisible
