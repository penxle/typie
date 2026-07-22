package co.typie.screen.editor.editor.toolbar

import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

internal enum class EditorKeyboardType {
  Software,
  Hardware,
}

internal enum class EditorImeInputOwner {
  Editor,
  Other,
}

internal sealed interface EditorKeyboardPresentation {
  data object Hidden : EditorKeyboardPresentation

  data object Showing : EditorKeyboardPresentation

  data class Shown(val settledImeBottom: Dp) : EditorKeyboardPresentation

  data object Hiding : EditorKeyboardPresentation
}

internal data class EditorKeyboardState(
  val type: EditorKeyboardType,
  val imeFrameVisible: Boolean = false,
  val imeHideEventVersion: Int = 0,
  val imeHideEventOwner: EditorImeInputOwner? = null,
  val presentation: EditorKeyboardPresentation = EditorKeyboardPresentation.Hidden,
  val hardwareKeyboardAttached: Boolean = type == EditorKeyboardType.Hardware,
) {
  val usesImeInset: Boolean
    get() = type == EditorKeyboardType.Software || imeFrameVisible

  val settledImeBottom: Dp?
    get() =
      when (val currentPresentation = presentation) {
        is EditorKeyboardPresentation.Shown -> currentPresentation.settledImeBottom
        EditorKeyboardPresentation.Hidden,
        EditorKeyboardPresentation.Hiding,
        EditorKeyboardPresentation.Showing -> null
      }
}

internal class EditorImeHideOwnershipTracker {
  private var visibleOwner: EditorImeInputOwner? = null
  private var hideEventOwner: EditorImeInputOwner? = null

  fun observeVisibleOwner(editorInputSessionActive: Boolean) {
    visibleOwner =
      if (editorInputSessionActive) {
        EditorImeInputOwner.Editor
      } else {
        EditorImeInputOwner.Other
      }
    hideEventOwner = null
  }

  fun beginHide(): EditorImeInputOwner? {
    if (hideEventOwner == null) {
      hideEventOwner = visibleOwner
    }
    return hideEventOwner
  }

  fun observe(
    presentation: EditorKeyboardPresentation,
    editorInputSessionActive: Boolean,
  ): EditorImeInputOwner? =
    when (presentation) {
      EditorKeyboardPresentation.Showing,
      is EditorKeyboardPresentation.Shown -> {
        observeVisibleOwner(editorInputSessionActive)
        null
      }
      EditorKeyboardPresentation.Hiding,
      EditorKeyboardPresentation.Hidden -> beginHide()
    }
}

@Composable
internal expect fun rememberEditorKeyboardState(
  isEditorInputSessionActive: () -> Boolean
): EditorKeyboardState

internal fun isImeVisible(imeBottom: Dp, safeBottomInset: Dp): Boolean = imeBottom > safeBottomInset

internal fun trustedImeBottomInset(rawImeBottom: Dp, keyboardState: EditorKeyboardState): Dp {
  if (!keyboardState.usesImeInset) {
    return 0.dp
  }

  val settledImeInset = keyboardState.settledImeBottom
  return if (settledImeInset != null && rawImeBottom > settledImeInset) {
    settledImeInset
  } else {
    rawImeBottom
  }
}

internal fun resolveKeyboardPresentation(
  imeBottom: Dp,
  animationSourceBottom: Dp,
  animationTargetBottom: Dp,
): EditorKeyboardPresentation =
  when {
    imeBottom <= 0.dp && animationTargetBottom <= 0.dp -> EditorKeyboardPresentation.Hidden
    animationTargetBottom > 0.dp && imeBottom < animationTargetBottom ->
      EditorKeyboardPresentation.Showing
    animationTargetBottom > 0.dp -> EditorKeyboardPresentation.Shown(animationTargetBottom)
    animationSourceBottom > 0.dp -> EditorKeyboardPresentation.Hiding
    imeBottom > 0.dp -> EditorKeyboardPresentation.Shown(imeBottom)
    else -> EditorKeyboardPresentation.Hidden
  }
