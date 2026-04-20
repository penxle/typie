@file:OptIn(
  ExperimentalForeignApi::class,
  ExperimentalComposeUiApi::class,
  ExperimentalContracts::class,
)

package co.typie.editor.compose

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.viewinterop.UIKitInteropProperties
import androidx.compose.ui.viewinterop.UIKitView
import kotlin.contracts.ExperimentalContracts
import kotlin.math.roundToInt
import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.cValue
import kotlinx.cinterop.useContents
import platform.CoreGraphics.CGRect
import platform.CoreGraphics.CGSizeMake
import platform.QuartzCore.CAMetalLayer
import platform.QuartzCore.CATransaction
import platform.UIKit.UIScreen
import platform.UIKit.UIView
import swiftPMImport.co.typie.compose.MetalSurfaceBridge

private class MetalSurfaceView(private val metalLayer: CAMetalLayer) :
  UIView(frame = cValue<CGRect> {}) {
  var onLayoutChanged: (() -> Unit)? = null
  private var lastWidth = 0.0
  private var lastHeight = 0.0

  init {
    opaque = true

    metalLayer.framebufferOnly = false
    metalLayer.opaque = true
    metalLayer.contentsScale = UIScreen.mainScreen.scale
    metalLayer.presentsWithTransaction = true
    layer.addSublayer(metalLayer)
  }

  override fun layoutSubviews() {
    super.layoutSubviews()
    val w = bounds.useContents { size.width }
    val h = bounds.useContents { size.height }

    CATransaction.begin()
    CATransaction.setDisableActions(true)
    metalLayer.frame = bounds
    val scale = metalLayer.contentsScale
    metalLayer.drawableSize =
      CGSizeMake((w * scale).roundToInt().toDouble(), (h * scale).roundToInt().toDouble())
    if (w > 0.0 && h > 0.0 && (w != lastWidth || h != lastHeight)) {
      lastWidth = w
      lastHeight = h
      onLayoutChanged?.invoke()
    }
    CATransaction.commit()
  }
}

@Composable
internal actual fun Surface(
  modifier: Modifier,
  onAttach: (handle: Long) -> Unit,
  onDetach: () -> Unit,
  onResize: () -> Unit,
) {
  val metalLayer = remember { CAMetalLayer() }

  UIKitView(
    factory = {
      MetalSurfaceView(metalLayer).also { view ->
        view.onLayoutChanged = onResize
        onAttach(MetalSurfaceBridge.pointerOf(metalLayer))
      }
    },
    modifier = modifier,
    update = { view -> view.onLayoutChanged = onResize },
    onRelease = { onDetach() },
    properties = UIKitInteropProperties(interactionMode = null, placedAsOverlay = false),
  )
}
