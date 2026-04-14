package co.typie.ui.component.toast

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInRoot
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

@Composable
fun ToastAnchor(inset: Dp = 12.dp, modifier: Modifier = Modifier) {
  val toast = LocalToast.current

  DisposableEffect(Unit) { onDispose { toast.anchorY = null } }

  Box(
    modifier.padding(vertical = inset).fillMaxWidth().height(0.dp).onGloballyPositioned {
      coordinates ->
      toast.anchorY = coordinates.positionInRoot().y
    }
  )
}
