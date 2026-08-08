@file:OptIn(kotlinx.cinterop.ExperimentalForeignApi::class)

package co.typie.platform

import androidx.compose.runtime.Composable
import swiftPMImport.co.typie.compose.SoftwareKeyboardPresentationBridge

private val IosSoftwareKeyboardPresentationDriverFactory =
  SoftwareKeyboardPresentationDriverFactory { onInvalidated ->
    val bridge = SoftwareKeyboardPresentationBridge()
    bridge.onInvalidated = onInvalidated
    if (!bridge.acquire()) {
      bridge.dispose()
      null
    } else {
      IosSoftwareKeyboardPresentationDriver(bridge)
    }
  }

private class IosSoftwareKeyboardPresentationDriver(
  private var bridge: SoftwareKeyboardPresentationBridge?
) : SoftwareKeyboardPresentationDriver {
  override fun updateHiddenProgress(progress: Float) {
    checkNotNull(bridge).hiddenProgress = progress.toDouble()
  }

  override fun finish(endpoint: SoftwareKeyboardPresentationEndpoint, onAccepted: () -> Unit) {
    val current = checkNotNull(bridge)
    current.onAccepted = {
      if (bridge === current) {
        bridge = null
        onAccepted()
      }
    }
    when (endpoint) {
      SoftwareKeyboardPresentationEndpoint.Shown -> current.finishShown()
      SoftwareKeyboardPresentationEndpoint.Hidden -> current.finishHidden()
    }
  }

  override fun dispose() {
    val current = bridge ?: return
    bridge = null
    current.dispose()
  }
}

@Composable
internal actual fun rememberSoftwareKeyboardPresentationDriverFactory():
  SoftwareKeyboardPresentationDriverFactory = IosSoftwareKeyboardPresentationDriverFactory
