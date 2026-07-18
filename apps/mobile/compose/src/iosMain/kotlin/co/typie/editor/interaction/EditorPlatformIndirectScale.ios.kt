@file:OptIn(ExperimentalForeignApi::class)

package co.typie.editor.interaction

import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.composed
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.input.pointer.PointerKeyboardModifiers
import androidx.compose.ui.input.pointer.isMetaPressed
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.uikit.LocalUIViewController
import kotlinx.cinterop.ExperimentalForeignApi
import swiftPMImport.co.typie.compose.EditorIndirectScaleGestureBridge

internal actual fun Modifier.editorPlatformIndirectScale(
  bridge: EditorPlatformIndirectScaleBridge,
  enabled: Boolean,
  density: Float,
): Modifier = composed {
  val view = LocalUIViewController.current.view
  val gestureBridge = remember(view) { EditorIndirectScaleGestureBridge(view = view) }
  var boundsInRoot by remember { mutableStateOf(Rect.Zero) }

  SideEffect {
    gestureBridge.onShouldBegin = { x, y ->
      if (!enabled || density <= 0f) {
        false
      } else {
        boundsInRoot.contains(Offset(x.toFloat() * density, y.toFloat() * density))
      }
    }
    gestureBridge.onBegin = bridge::begin
    gestureBridge.onScale = { x, y, scaleFactor ->
      bridge.update(Offset(x.toFloat() * density, y.toFloat() * density), scaleFactor.toFloat())
    }
    gestureBridge.onEnd = bridge::end
    if (!enabled) {
      gestureBridge.endActive()
    }
  }
  DisposableEffect(gestureBridge) { onDispose { gestureBridge.dispose() } }

  onGloballyPositioned { coordinates -> boundsInRoot = coordinates.boundsInRoot() }
}

internal actual fun PointerKeyboardModifiers.isEditorIndirectZoomModifierPressed(): Boolean =
  isMetaPressed
