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
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import co.typie.ext.ime
import co.typie.ext.imePadding
import co.typie.ui.component.ResponsiveContainerDefaults
import co.typie.ui.component.Text
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

private val ToolbarOuterVerticalPadding = 12.dp

@Composable
internal fun EditorToolbarHost(
  bodyFocused: Boolean,
  modifier: Modifier = Modifier,
  onVisibleTopChanged: (Float?) -> Unit,
) {
  val density = LocalDensity.current
  val imeVisible = WindowInsets.ime.asPaddingValues().calculateBottomPadding() > 0.dp
  val visible = shouldShowEditorToolbar(bodyFocused = bodyFocused, imeVisible = imeVisible)

  LaunchedEffect(visible) {
    if (!visible) {
      onVisibleTopChanged(null)
    }
  }

  AnimatedVisibility(
    visible = visible,
    enter = fadeIn(),
    exit = fadeOut(),
    modifier = modifier.fillMaxWidth(),
  ) {
    Box(
      modifier =
        Modifier.fillMaxWidth()
          .imePadding()
          .padding(horizontal = 16.dp, vertical = ToolbarOuterVerticalPadding)
          .onGloballyPositioned { coordinates ->
            if (visible) {
              onVisibleTopChanged(coordinates.boundsInRoot().top / density.density)
            }
          },
      contentAlignment = Alignment.BottomCenter,
    ) {
      Box(
        modifier =
          Modifier.widthIn(max = ResponsiveContainerDefaults.MaxWidth)
            .fillMaxWidth()
            .height(48.dp)
            .background(AppTheme.colors.surfaceDefault, AppShapes.rounded(AppShapes.md))
            .border(1.dp, AppTheme.colors.borderDefault, AppShapes.rounded(AppShapes.md)),
        contentAlignment = Alignment.Center,
      ) {
        // TODO(editor-parity): Replace the placeholder bar with bottom/floating/secondary toolbars
        // that react to editor selection state and keyboard visibility.
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
