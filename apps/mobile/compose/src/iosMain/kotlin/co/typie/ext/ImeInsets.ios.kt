@file:OptIn(kotlinx.cinterop.ExperimentalForeignApi::class)

package co.typie.ext

import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import platform.Foundation.NSNotification
import platform.Foundation.NSNotificationCenter
import platform.Foundation.NSOperationQueue
import platform.UIKit.UIKeyboardDidChangeFrameNotification
import platform.UIKit.UIKeyboardDidHideNotification
import platform.UIKit.UIKeyboardDidShowNotification
import platform.UIKit.UIKeyboardWillChangeFrameNotification
import platform.UIKit.UIKeyboardWillHideNotification
import swiftPMImport.co.typie.compose.EditorKeyboardBridge

@Composable
internal actual fun rememberTrustedImeBottomInset(): Dp {
  val density = LocalDensity.current
  val rawImeBottom = with(density) { WindowInsets.ime.getBottom(this).toDp() }
  var settledImeBottom by remember { mutableStateOf<Dp?>(null) }

  DisposableEffect(Unit) {
    fun updateSettledImeBottom(notification: NSNotification?) {
      settledImeBottom =
        notification?.let(EditorKeyboardBridge::imeVisibleHeightWithNotification)?.dp?.takeIf {
          it > 0.dp
        }
    }

    val center = NSNotificationCenter.defaultCenter
    val willChangeFrameObserver =
      center.addObserverForName(
        name = UIKeyboardWillChangeFrameNotification,
        `object` = null,
        queue = NSOperationQueue.mainQueue,
      ) {
        updateSettledImeBottom(it)
      }
    val didChangeFrameObserver =
      center.addObserverForName(
        name = UIKeyboardDidChangeFrameNotification,
        `object` = null,
        queue = NSOperationQueue.mainQueue,
      ) {
        updateSettledImeBottom(it)
      }
    val didShowObserver =
      center.addObserverForName(
        name = UIKeyboardDidShowNotification,
        `object` = null,
        queue = NSOperationQueue.mainQueue,
      ) {
        updateSettledImeBottom(it)
      }
    val willHideObserver =
      center.addObserverForName(
        name = UIKeyboardWillHideNotification,
        `object` = null,
        queue = NSOperationQueue.mainQueue,
      ) {
        settledImeBottom = null
      }
    val didHideObserver =
      center.addObserverForName(
        name = UIKeyboardDidHideNotification,
        `object` = null,
        queue = NSOperationQueue.mainQueue,
      ) {
        settledImeBottom = null
      }

    onDispose {
      center.removeObserver(willChangeFrameObserver)
      center.removeObserver(didChangeFrameObserver)
      center.removeObserver(didShowObserver)
      center.removeObserver(willHideObserver)
      center.removeObserver(didHideObserver)
    }
  }

  return trustedImeBottomInset(rawImeBottom = rawImeBottom, settledImeBottom = settledImeBottom)
}
