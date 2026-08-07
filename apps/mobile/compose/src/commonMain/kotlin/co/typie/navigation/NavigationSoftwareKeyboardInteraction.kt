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

  fun start() {
    if (session != null) return
    session = controller.acquire()
  }

  fun updateHiddenProgress(progress: Float) {
    session?.updateHiddenProgress(progress)
  }

  fun restore() {
    val activeSession = session ?: return
    session = null
    activeSession.finish(SoftwareKeyboardPresentationEndpoint.Shown)
  }

  suspend fun hideAndAwaitResolution(): SoftwareKeyboardInteractionResolution? {
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
    session?.dispose()
    session = null
  }
}
