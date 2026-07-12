package co.typie.platform

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInWindow
import androidx.compose.ui.platform.LocalDensity

interface Share {
  suspend fun share(bytes: ByteArray, mimeType: String, anchor: ShareAnchor?): Boolean

  suspend fun share(text: String, anchor: ShareAnchor?): Boolean
}

data class ShareAnchor(val x: Double, val y: Double, val width: Double, val height: Double)

class ShareAnchorState internal constructor() {
  internal var density = 1f

  var value: ShareAnchor? = null
    private set

  val modifier: Modifier = Modifier.onGloballyPositioned { coordinates ->
    val position = coordinates.positionInWindow()
    val size = coordinates.size
    value =
      ShareAnchor(
        x = (position.x / density).toDouble(),
        y = (position.y / density).toDouble(),
        width = (size.width / density).toDouble(),
        height = (size.height / density).toDouble(),
      )
  }
}

@Composable
fun rememberShareAnchor(): ShareAnchorState {
  val state = remember { ShareAnchorState() }
  state.density = LocalDensity.current.density
  return state
}
