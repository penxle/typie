package co.typie.ui.component.bottomsheet

import androidx.compose.runtime.Composable
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.launch

internal fun showBottomSheetFromPopoverAction(
  closePopover: () -> Unit,
  presenterScope: CoroutineScope,
  bottomSheetHost: BottomSheetHostState,
  content: @Composable BottomSheetScope<Unit>.() -> Unit,
) {
  closePopover()
  presenterScope.launch(start = CoroutineStart.UNDISPATCHED) {
    try {
      bottomSheetHost.show(content)
    } catch (_: CancellationException) {}
  }
}
