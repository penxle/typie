package co.typie.screen.editor.editor.overlay

import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.composed
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.isSpecified
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInWindow
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.uikit.LocalUIViewController
import androidx.compose.ui.uikit.utils.CMPTextLoupeSession
import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.readValue
import kotlinx.cinterop.useContents
import org.jetbrains.skiko.OS
import org.jetbrains.skiko.OSVersion
import org.jetbrains.skiko.available
import platform.CoreGraphics.CGPointMake
import platform.CoreGraphics.CGRectZero
import platform.UIKit.UIView

internal actual val EditorNativeMagnifierAvailable: Boolean
  get() = available(OS.Ios to OSVersion(major = 17))

internal actual fun Modifier.editorNativeMagnifier(placement: EditorMagnifierPlacement?): Modifier =
  composed {
    if (!EditorNativeMagnifierAvailable) {
      return@composed this
    }

    val density = LocalDensity.current
    val view = LocalUIViewController.current.view
    val session = remember { IosEditorNativeMagnifierSession() }
    var anchorPositionInWindow by remember { mutableStateOf(Offset.Unspecified) }

    SideEffect {
      session.update(
        view = view,
        density = density.density,
        anchorPositionInWindow = anchorPositionInWindow,
        placement = placement,
      )
    }
    DisposableEffect(session) { onDispose { session.dismiss() } }

    onGloballyPositioned { coordinates -> anchorPositionInWindow = coordinates.positionInWindow() }
  }

@OptIn(ExperimentalForeignApi::class)
private class IosEditorNativeMagnifierSession {
  private var view: UIView? = null
  private var loupeSession: CMPTextLoupeSession? = null

  fun update(
    view: UIView,
    density: Float,
    anchorPositionInWindow: Offset,
    placement: EditorMagnifierPlacement?,
  ) {
    if (
      placement == null ||
        density <= 0f ||
        !anchorPositionInWindow.isSpecified ||
        !placement.sourceCenter.isSpecified
    ) {
      dismiss()
      return
    }

    if (this.view !== view) {
      dismiss()
      this.view = view
    }

    val window =
      view.window
        ?: run {
          dismiss()
          return
        }
    val sourceCenterInWindow = anchorPositionInWindow + placement.sourceCenter
    val sourceCenterInView =
      view
        .convertPoint(
          point =
            CGPointMake(
              sourceCenterInWindow.x.toDouble() / density,
              sourceCenterInWindow.y.toDouble() / density,
            ),
          fromCoordinateSpace = window.coordinateSpace(),
        )
        .useContents {
          val layerOffset =
            view.layer.affineTransform().useContents {
              Offset(x = tx.toFloat() * density, y = ty.toFloat() * density)
            }
          Offset(x = x.toFloat() * density, y = y.toFloat() * density) + layerOffset
        }
    val sourcePoint =
      CGPointMake(
        sourceCenterInView.x.toDouble() / density,
        sourceCenterInView.y.toDouble() / density,
      )
    val session =
      loupeSession
        ?: CMPTextLoupeSession.beginLoupeSessionAtPoint(
            point = sourcePoint,
            fromSelectionWidgetView = null,
            inView = view,
          )
          ?.also { loupeSession = it }

    session?.moveToPoint(
      point = sourcePoint,
      withCaretRect = CGRectZero.readValue(),
      trackingCaret = false,
    )
  }

  fun dismiss() {
    loupeSession?.invalidate()
    loupeSession = null
  }
}
