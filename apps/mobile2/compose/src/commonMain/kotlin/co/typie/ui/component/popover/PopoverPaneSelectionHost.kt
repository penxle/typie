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
internal fun PopoverPaneSelectionHost(
  scope: PopoverScope,
  pane:
    @Composable
    context(PopoverScope)
    () -> Unit,
) {
  val paneSelectionState = rememberPopoverPaneSelectionState()
  val edgeAutoScrollState = LocalPopoverPaneEdgeAutoScrollState.current
  val itemRadius = PopoverDefaults.ExpandedRadius - PopoverDefaults.PanePadding
  var paneCoordinates by remember { mutableStateOf<LayoutCoordinates?>(null) }
  val selectionInputModifier =
    if (scope.acceptsInput) {
      rememberPopoverPaneSelectionInputModifier(
        enabled = true,
        positionInWindow = { localPosition -> paneCoordinates?.localToWindow(localPosition) },
        selectionState = paneSelectionState,
        edgeAutoScrollState = edgeAutoScrollState,
      )
    } else {
      Modifier
    }

  LaunchedEffect(scope.acceptsInput, scope.pressGestureSession) {
    if (!scope.acceptsInput) {
      paneSelectionState.reset()
      edgeAutoScrollState?.stop()
      return@LaunchedEffect
    }

    val session = scope.pressGestureSession
    paneSelectionState.syncSharedSession(session)
    if (session?.isArmed == true && !session.isReleased) {
      edgeAutoScrollState?.update(pointerPosition = session.positionInWindow) {
        paneSelectionState.syncSharedSession(scope.pressGestureSession)
      }
    } else {
      edgeAutoScrollState?.stop()
    }
  }

  DisposableEffect(Unit) {
    onDispose {
      paneSelectionState.reset()
      edgeAutoScrollState?.stop()
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
      context(scope) { pane() }
    }
  }
}
