@file:OptIn(kotlinx.cinterop.ExperimentalForeignApi::class)

package co.typie.screen.editor.editor.toolbar

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.unit.dp
import platform.Foundation.NSNotificationCenter
import platform.Foundation.NSOperationQueue
import platform.GameController.GCKeyboardDidConnectNotification
import platform.GameController.GCKeyboardDidDisconnectNotification
import platform.UIKit.UIKeyboardDidChangeFrameNotification
import platform.UIKit.UIKeyboardDidHideNotification
import platform.UIKit.UIKeyboardDidShowNotification
import platform.UIKit.UIKeyboardWillChangeFrameNotification
import platform.UIKit.UIKeyboardWillHideNotification
import swiftPMImport.co.typie.compose.EditorKeyboardBridge

@Composable
internal actual fun rememberEditorKeyboardState(
  isEditorInputSessionActive: () -> Boolean
): EditorKeyboardState {
  val editorInputSessionActive = isEditorInputSessionActive()
  val currentEditorInputSessionActive by rememberUpdatedState(editorInputSessionActive)
  val imeHideOwnershipTracker = remember { EditorImeHideOwnershipTracker() }
  var hardwareKeyboardMode by remember { mutableStateOf(isEditorHardwareKeyboardConnected()) }
  var imeFrameVisible by remember { mutableStateOf(false) }
  var imeHideEventVersion by remember { mutableIntStateOf(0) }
  var imeHideEventOwner by remember { mutableStateOf<EditorImeInputOwner?>(null) }
  var presentation by remember {
    mutableStateOf<EditorKeyboardPresentation>(EditorKeyboardPresentation.Hidden)
  }

  fun syncKeyboardState() {
    hardwareKeyboardMode = isEditorHardwareKeyboardConnected()
  }

  fun syncKeyboardFrame(
    notification: platform.Foundation.NSNotification?,
    settlesImeBottom: Boolean,
  ) {
    val targetBottom =
      notification?.let(EditorKeyboardBridge::imeVisibleHeightWithNotification)?.dp ?: 0.dp
    if (targetBottom > 0.dp) {
      imeHideOwnershipTracker.observeVisibleOwner(currentEditorInputSessionActive)
      imeHideEventOwner = null
    } else if (
      presentation != EditorKeyboardPresentation.Hidden &&
        presentation != EditorKeyboardPresentation.Hiding
    ) {
      imeHideEventOwner = imeHideOwnershipTracker.beginHide()
    }
    if (settlesImeBottom) {
      presentation =
        if (targetBottom > 0.dp) {
          EditorKeyboardPresentation.Shown(targetBottom)
        } else {
          EditorKeyboardPresentation.Hidden
        }
    } else {
      presentation =
        if (targetBottom > 0.dp) {
          when (presentation) {
            EditorKeyboardPresentation.Hidden,
            EditorKeyboardPresentation.Hiding -> EditorKeyboardPresentation.Showing
            EditorKeyboardPresentation.Showing,
            is EditorKeyboardPresentation.Shown -> presentation
          }
        } else {
          when (presentation) {
            EditorKeyboardPresentation.Hidden -> EditorKeyboardPresentation.Hidden
            EditorKeyboardPresentation.Showing,
            is EditorKeyboardPresentation.Shown,
            EditorKeyboardPresentation.Hiding -> EditorKeyboardPresentation.Hiding
          }
        }
    }
    imeFrameVisible = targetBottom > 0.dp
    syncKeyboardState()
  }

  SideEffect {
    if (
      presentation == EditorKeyboardPresentation.Showing ||
        presentation is EditorKeyboardPresentation.Shown
    ) {
      imeHideOwnershipTracker.observeVisibleOwner(editorInputSessionActive)
      if (imeHideEventOwner != null) {
        imeHideEventOwner = null
      }
    }
  }

  DisposableEffect(Unit) {
    val center = NSNotificationCenter.defaultCenter
    val connectObserver =
      center.addObserverForName(
        name = GCKeyboardDidConnectNotification,
        `object` = null,
        queue = NSOperationQueue.mainQueue,
      ) {
        syncKeyboardState()
        if (hardwareKeyboardMode) {
          imeFrameVisible = false
          presentation = EditorKeyboardPresentation.Hidden
        }
      }
    val disconnectObserver =
      center.addObserverForName(
        name = GCKeyboardDidDisconnectNotification,
        `object` = null,
        queue = NSOperationQueue.mainQueue,
      ) {
        syncKeyboardState()
        if (hardwareKeyboardMode) {
          imeFrameVisible = false
          presentation = EditorKeyboardPresentation.Hidden
        }
      }
    val frameObserver =
      center.addObserverForName(
        name = UIKeyboardWillChangeFrameNotification,
        `object` = null,
        queue = NSOperationQueue.mainQueue,
      ) {
        syncKeyboardFrame(it, settlesImeBottom = false)
      }
    val didChangeFrameObserver =
      center.addObserverForName(
        name = UIKeyboardDidChangeFrameNotification,
        `object` = null,
        queue = NSOperationQueue.mainQueue,
      ) {
        syncKeyboardFrame(it, settlesImeBottom = true)
      }
    val didShowObserver =
      center.addObserverForName(
        name = UIKeyboardDidShowNotification,
        `object` = null,
        queue = NSOperationQueue.mainQueue,
      ) {
        syncKeyboardFrame(it, settlesImeBottom = true)
      }
    val hideObserver =
      center.addObserverForName(
        name = UIKeyboardWillHideNotification,
        `object` = null,
        queue = NSOperationQueue.mainQueue,
      ) {
        if (presentation != EditorKeyboardPresentation.Hiding) {
          imeHideEventOwner = imeHideOwnershipTracker.beginHide()
        }
        presentation = EditorKeyboardPresentation.Hiding
        imeFrameVisible = false
        imeHideEventVersion++
        syncKeyboardState()
      }
    val didHideObserver =
      center.addObserverForName(
        name = UIKeyboardDidHideNotification,
        `object` = null,
        queue = NSOperationQueue.mainQueue,
      ) {
        presentation = EditorKeyboardPresentation.Hidden
        imeFrameVisible = false
        syncKeyboardState()
      }

    onDispose {
      center.removeObserver(connectObserver)
      center.removeObserver(disconnectObserver)
      center.removeObserver(frameObserver)
      center.removeObserver(didChangeFrameObserver)
      center.removeObserver(didShowObserver)
      center.removeObserver(hideObserver)
      center.removeObserver(didHideObserver)
    }
  }

  return EditorKeyboardState(
    type =
      if (hardwareKeyboardMode) {
        EditorKeyboardType.Hardware
      } else {
        EditorKeyboardType.Software
      },
    imeFrameVisible = imeFrameVisible,
    imeHideEventVersion = imeHideEventVersion,
    imeHideEventOwner = imeHideEventOwner,
    presentation = presentation,
  )
}

private fun isEditorHardwareKeyboardConnected(): Boolean =
  EditorKeyboardBridge.isInHardwareKeyboardMode()
