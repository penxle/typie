package co.typie.screen.editor.editor.toolbar

import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.ext.trustedImeBottomInset as trustedSettledImeBottomInset
import co.typie.platform.SoftwareKeyboardInteractionResolution
import co.typie.platform.SoftwareKeyboardInteractionState

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

internal class EditorKeyboardInteractionResolver {
  private var lastSemanticState: EditorKeyboardState? = null
  private var retainedInteraction: RetainedKeyboardInteraction? = null
  private var nativeHideVersionOffset = 0

  fun resolve(
    nativeState: EditorKeyboardState,
    interactionState: SoftwareKeyboardInteractionState,
  ): EditorKeyboardState {
    val normalizedNativeState = nativeState.withNormalizedHideVersion()
    val activeInteractionId = interactionState.activeInteractionId
    if (activeInteractionId != null) {
      if (retainedInteraction?.interactionId != activeInteractionId) {
        lastSemanticState
          ?.takeIf { it.presentation is EditorKeyboardPresentation.Shown }
          ?.let {
            retainedInteraction =
              RetainedKeyboardInteraction(
                interactionId = activeInteractionId,
                resolutionVersion = interactionState.resolutionVersion,
                semanticState = it,
              )
          }
      }
      return retainOrRecord(normalizedNativeState)
    }

    val retained = retainedInteraction
    if (retained != null && interactionState.resolutionVersion > retained.resolutionVersion) {
      return when (interactionState.lastResolution) {
        SoftwareKeyboardInteractionResolution.Shown -> {
          if (nativeState.presentation is EditorKeyboardPresentation.Shown) {
            nativeHideVersionOffset =
              (nativeState.imeHideEventVersion - retained.semanticState.imeHideEventVersion)
                .coerceAtLeast(0)
            retainedInteraction = null
            record(
              nativeState.copy(imeHideEventVersion = retained.semanticState.imeHideEventVersion)
            )
          } else {
            retainOrRecord(normalizedNativeState)
          }
        }
        SoftwareKeyboardInteractionResolution.Hidden -> {
          if (
            nativeState.presentation == EditorKeyboardPresentation.Hiding ||
              nativeState.presentation == EditorKeyboardPresentation.Hidden
          ) {
            retainedInteraction = null
            record(normalizedNativeState)
          } else {
            retainOrRecord(normalizedNativeState)
          }
        }
        SoftwareKeyboardInteractionResolution.Aborted -> {
          retainedInteraction = null
          record(normalizedNativeState)
        }
        null -> retainOrRecord(normalizedNativeState)
      }
    }

    return record(normalizedNativeState)
  }

  private fun retainOrRecord(nativeState: EditorKeyboardState): EditorKeyboardState =
    record(retainedInteraction?.semanticState?.retainOver(nativeState) ?: nativeState)

  private fun record(state: EditorKeyboardState): EditorKeyboardState {
    lastSemanticState = state
    return state
  }

  private fun EditorKeyboardState.withNormalizedHideVersion(): EditorKeyboardState =
    copy(imeHideEventVersion = (imeHideEventVersion - nativeHideVersionOffset).coerceAtLeast(0))

  private fun EditorKeyboardState.retainOver(
    nativeState: EditorKeyboardState
  ): EditorKeyboardState =
    nativeState.copy(
      imeFrameVisible = imeFrameVisible,
      imeHideEventVersion = imeHideEventVersion,
      imeHideEventOwner = imeHideEventOwner,
      presentation = presentation,
    )

  private data class RetainedKeyboardInteraction(
    val interactionId: Long,
    val resolutionVersion: Long,
    val semanticState: EditorKeyboardState,
  )
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

  return trustedSettledImeBottomInset(
    rawImeBottom = rawImeBottom,
    settledImeBottom = keyboardState.settledImeBottom,
  )
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
