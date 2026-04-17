package co.typie.ui.component.popover

import androidx.compose.foundation.layout.Box
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.layout.onGloballyPositioned

@Composable
internal fun SelectablePaneHost(
  acceptsInput: Boolean,
  pressGestureSession: PressGestureSession?,
  content: @Composable () -> Unit,
) {
  val autoScrollController = LocalPopoverPaneAutoScrollController.current
  val itemRadius = PopoverDefaults.ExpandedRadius - PopoverDefaults.PanePadding
  val paneSelectionState = rememberPopoverPaneSelectionState()
  var paneCoordinates by remember { mutableStateOf<LayoutCoordinates?>(null) }
  val selectionInputModifier =
    if (acceptsInput) {
      rememberPopoverPaneSelectionInputModifier(
        enabled = true,
        positionInWindow = { localPosition -> paneCoordinates?.localToWindow(localPosition) },
        selectionState = paneSelectionState,
        autoScrollController = autoScrollController,
      )
    } else {
      Modifier
    }

  LaunchedEffect(acceptsInput, pressGestureSession) {
    if (!acceptsInput) {
      paneSelectionState.reset()
      autoScrollController?.pointer = null
      return@LaunchedEffect
    }

    paneSelectionState.syncSharedSession(pressGestureSession)
    if (pressGestureSession?.isArmed == true && !pressGestureSession.isReleased) {
      autoScrollController?.pointer = pressGestureSession.positionInWindow
    } else {
      autoScrollController?.pointer = null
    }
  }

  DisposableEffect(Unit) {
    onDispose {
      paneSelectionState.reset()
      autoScrollController?.pointer = null
    }
  }

  CompositionLocalProvider(LocalPopoverPaneSelectionState provides paneSelectionState) {
    Box(
      modifier =
        Modifier.onGloballyPositioned { coordinates ->
            paneCoordinates = coordinates
            paneSelectionState.updatePaneCoordinates(coordinates)
          }
          .then(selectionInputModifier)
    ) {
      PopoverPaneSelectionIndicator(
        activeBoundsInPane = paneSelectionState.activeItemBoundsInPane,
        itemRadius = itemRadius,
      )
      content()
    }
  }
}
