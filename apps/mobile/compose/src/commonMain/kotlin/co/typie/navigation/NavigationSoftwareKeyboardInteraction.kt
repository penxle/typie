package co.typie.navigation

import androidx.compose.runtime.snapshotFlow
import co.typie.platform.SoftwareKeyboardInteractionResolution
import co.typie.platform.SoftwareKeyboardPresentationController
import co.typie.platform.SoftwareKeyboardPresentationEndpoint
import co.typie.platform.SoftwareKeyboardPresentationSession
import kotlinx.coroutines.flow.first

internal class NavigationSoftwareKeyboardInteraction(
  private val controller: SoftwareKeyboardPresentationController
) {
  private var session: SoftwareKeyboardPresentationSession? = null
  private var hiddenProgressContinuationStart: Float? = null

  fun start() {
    if (session != null) return
    hiddenProgressContinuationStart = null
    session = controller.acquire()
  }

  fun updateHiddenProgress(progress: Float) {
    val continuationStart = hiddenProgressContinuationStart
    val effectiveProgress =
      if (continuationStart == null) {
        progress
      } else {
        continuationStart + (1f - continuationStart) * progress.coerceIn(0f, 1f)
      }
    session?.updateHiddenProgress(effectiveProgress)
  }

  fun continueFromHiddenProgress(progress: Float) {
    if (session == null) return
    hiddenProgressContinuationStart = progress.coerceIn(0f, 1f)
  }

  fun restore() {
    hiddenProgressContinuationStart = null
    val activeSession = session ?: return
    session = null
    activeSession.finish(SoftwareKeyboardPresentationEndpoint.Shown)
  }

  suspend fun hideAndAwaitResolution(): SoftwareKeyboardInteractionResolution? {
    hiddenProgressContinuationStart = null
    val activeSession = session ?: return null
    session = null
    val interactionId = activeSession.interactionId
    activeSession.finish(SoftwareKeyboardPresentationEndpoint.Hidden)
    val resolvedState =
      snapshotFlow { controller.interactionState }.first { it.activeInteractionId != interactionId }
    return resolvedState.lastResolution.takeIf {
      resolvedState.lastResolvedInteractionId == interactionId
    }
  }

  fun dispose() {
    hiddenProgressContinuationStart = null
    session?.dispose()
    session = null
  }
}
