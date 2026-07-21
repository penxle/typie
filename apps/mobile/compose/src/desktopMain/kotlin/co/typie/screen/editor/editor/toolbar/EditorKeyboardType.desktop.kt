package co.typie.screen.editor.editor.toolbar

import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.dev.DesktopDebugKeyboard
import co.typie.ext.ime

@Composable
internal actual fun rememberEditorKeyboardState(
  isEditorInputSessionActive: () -> Boolean
): EditorKeyboardState {
  val density = LocalDensity.current
  val imeBottom = with(density) { WindowInsets.ime.getBottom(this).toDp() }
  val imeHideOwnershipTracker = remember { EditorImeHideOwnershipTracker() }
  return resolveDesktopEditorKeyboardState(
      hardwareKeyboardConnected = DesktopDebugKeyboard.hardwareKeyboardConnected,
      imeBottom = imeBottom,
    )
    .copy(
      imeHideEventOwner =
        imeHideOwnershipTracker.observe(
          visible = imeBottom > 0.dp,
          editorInputSessionActive = isEditorInputSessionActive(),
        )
    )
}

internal fun resolveDesktopEditorKeyboardState(
  hardwareKeyboardConnected: Boolean,
  imeBottom: Dp,
): EditorKeyboardState {
  if (hardwareKeyboardConnected) {
    return EditorKeyboardState(
      type = EditorKeyboardType.Hardware,
      imeFrameVisible = false,
      presentation = EditorKeyboardPresentation.Hidden,
    )
  }

  return EditorKeyboardState(
    type = EditorKeyboardType.Software,
    imeFrameVisible = imeBottom > 0.dp,
    presentation =
      resolveKeyboardPresentation(
        imeBottom = imeBottom,
        animationSourceBottom = 0.dp,
        animationTargetBottom = 0.dp,
      ),
  )
}
