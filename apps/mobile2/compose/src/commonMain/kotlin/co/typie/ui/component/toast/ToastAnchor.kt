package co.typie.ui.component.toast

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInRoot
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

@Composable
fun ToastAnchor(bottom: Dp = 12.dp, modifier: Modifier = Modifier) {
  val toast = LocalToast.current
  val entry = remember { AnchorEntry() }

  DisposableEffect(Unit) {
    toast.registerAnchor(entry)
    onDispose { toast.unregisterAnchor(entry) }
  }

  Box(
    modifier.padding(bottom = bottom).fillMaxWidth().height(0.dp).onGloballyPositioned { coordinates
      ->
      entry.y = coordinates.positionInRoot().y
    }
  )
}
