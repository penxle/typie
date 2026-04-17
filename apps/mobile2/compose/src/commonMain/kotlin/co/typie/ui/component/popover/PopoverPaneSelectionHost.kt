package co.typie.ui.component.popover

import androidx.compose.runtime.Composable

@Composable
internal fun PopoverPaneSelectionHost(
  scope: PopoverScope,
  pane:
    @Composable
    context(PopoverScope)
    () -> Unit,
) {
  SelectablePaneHost(
    acceptsInput = scope.acceptsInput,
    pressGestureSession = scope.pressGestureSession,
  ) {
    context(scope) { pane() }
  }
}
