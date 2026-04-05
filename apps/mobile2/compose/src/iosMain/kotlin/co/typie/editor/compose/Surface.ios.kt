package co.typie.editor.compose

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.viewinterop.UIKitInteropProperties
import androidx.compose.ui.viewinterop.UIKitView
import kotlinx.cinterop.ExperimentalForeignApi
import platform.QuartzCore.CAMetalLayer
import platform.QuartzCore.CATransaction
import platform.UIKit.UIColor
import platform.UIKit.UIView
import swiftPMImport.co.typie.compose.MetalSurfaceBridge

@OptIn(ExperimentalForeignApi::class, ExperimentalComposeUiApi::class)
@Composable
internal actual fun Surface(
  modifier: Modifier,
  onAttach: (handle: Long) -> Unit,
  onDetach: () -> Unit,
) {
  val metalLayer = remember { CAMetalLayer() }

  UIKitView(
    factory = {
      val view = UIView()
      view.opaque = false
      view.backgroundColor = UIColor.clearColor
      view.layer.addSublayer(metalLayer)
      onAttach(MetalSurfaceBridge.pointerOf(metalLayer))
      view
    },
    modifier = modifier,
    update = { view ->
      CATransaction.begin()
      CATransaction.setDisableActions(true)
      metalLayer.frame = view.bounds
      CATransaction.commit()
    },
    onRelease = {
      onDetach()
    },
    properties = UIKitInteropProperties(
      interactionMode = null,
      placedAsOverlay = true,
    ),
  )
}
