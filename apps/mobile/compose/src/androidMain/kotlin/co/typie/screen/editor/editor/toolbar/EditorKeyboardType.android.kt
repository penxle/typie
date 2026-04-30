// cspell:ignore NOKEYS

package co.typie.screen.editor.editor.toolbar

import android.content.res.Configuration
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.imeAnimationSource
import androidx.compose.foundation.layout.imeAnimationTarget
import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalDensity

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal actual fun rememberEditorKeyboardState(): EditorKeyboardState {
  val configuration = LocalConfiguration.current
  val density = LocalDensity.current
  val imeBottom = WindowInsets.ime.getBottom(density)
  val imeAnimationSourceBottom = WindowInsets.imeAnimationSource.getBottom(density)
  val imeAnimationTargetBottom = WindowInsets.imeAnimationTarget.getBottom(density)
  val imeBottomDp = with(density) { imeBottom.toDp() }
  val imeAnimationSourceBottomDp = with(density) { imeAnimationSourceBottom.toDp() }
  val imeAnimationTargetBottomDp = with(density) { imeAnimationTargetBottom.toDp() }
  val presentation =
    resolveKeyboardPresentation(
      imeBottom = imeBottomDp,
      animationSourceBottom = imeAnimationSourceBottomDp,
      animationTargetBottom = imeAnimationTargetBottomDp,
    )
  val hardwareKeyboardVisible =
    configuration.keyboard != Configuration.KEYBOARD_NOKEYS &&
      configuration.hardKeyboardHidden != Configuration.HARDKEYBOARDHIDDEN_YES
  return EditorKeyboardState(
    type =
      if (hardwareKeyboardVisible) {
        EditorKeyboardType.Hardware
      } else {
        EditorKeyboardType.Software
      },
    imeFrameVisible = imeBottom > 0 || imeAnimationTargetBottom > 0,
    presentation = presentation,
  )
}
