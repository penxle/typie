// cspell:ignore NOKEYS

package co.typie.screen.editor.editor.toolbar

import android.content.res.Configuration
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.imeAnimationSource
import androidx.compose.foundation.layout.imeAnimationTarget
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalDensity
import co.typie.platform.LocalSoftwareKeyboardPresentationController
import co.typie.platform.rememberExternalKeyboardAttached

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal actual fun rememberEditorKeyboardState(
  isEditorInputSessionActive: () -> Boolean
): EditorKeyboardState {
  val configuration = LocalConfiguration.current
  val density = LocalDensity.current
  val keyboardInteractionResolver = remember { EditorKeyboardInteractionResolver() }
  val keyboardInteractionState =
    LocalSoftwareKeyboardPresentationController.current.interactionState
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
  val imeHideOwnershipTracker = remember { EditorImeHideOwnershipTracker() }
  val imeHideEventOwner =
    imeHideOwnershipTracker.observe(
      presentation = presentation,
      editorInputSessionActive = isEditorInputSessionActive(),
    )
  val hardwareKeyboardVisible =
    configuration.keyboard != Configuration.KEYBOARD_NOKEYS &&
      configuration.hardKeyboardHidden != Configuration.HARDKEYBOARDHIDDEN_YES
  val externalKeyboardAttached = rememberExternalKeyboardAttached()
  return keyboardInteractionResolver.resolve(
    nativeState =
      EditorKeyboardState(
        type =
          if (hardwareKeyboardVisible) {
            EditorKeyboardType.Hardware
          } else {
            EditorKeyboardType.Software
          },
        imeFrameVisible = imeBottom > 0 || imeAnimationTargetBottom > 0,
        imeHideEventOwner = imeHideEventOwner,
        presentation = presentation,
        hardwareKeyboardAttached = hardwareKeyboardVisible || externalKeyboardAttached,
      ),
    interactionState = keyboardInteractionState,
  )
}

internal actual fun endInputMethodComposition() = Unit
