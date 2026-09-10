package co.typie.ui.component.popover

import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.onGloballyPositioned
import co.typie.ext.clickable
import co.typie.ui.input.hoverFeedback
import co.typie.ui.theme.AppShapes

data class PopoverListItem(
  val content: @Composable () -> Unit,
  val onSelected: () -> Unit,
  val enabled: Boolean = true,
)

@Composable
fun PopoverList(items: List<PopoverListItem>, acceptsInput: Boolean = true) {
  Column(modifier = Modifier.fillMaxWidth()) {
    items.forEach { item ->
      PopoverPaneSelectableItem(
        enabled = item.enabled && acceptsInput,
        onSelected = item.onSelected,
      ) {
        Box(Modifier.graphicsLayer { alpha = if (item.enabled) 1f else 0.4f }) { item.content() }
      }
    }
  }
}

@Composable
private fun PopoverPaneSelectableItem(
  enabled: Boolean,
  onSelected: () -> Unit,
  content: @Composable () -> Unit,
) {
  val paneSelectionState = LocalPopoverPaneSelectionState.current
  val registrationKey = remember { Any() }
  val interactionSource = remember { MutableInteractionSource() }
  val latestOnSelected = rememberUpdatedState(onSelected)

  DisposableEffect(paneSelectionState, registrationKey) {
    paneSelectionState?.registerItem(
      key = registrationKey,
      enabled = enabled,
      onSelected = { latestOnSelected.value() },
    )
    onDispose { paneSelectionState?.unregisterItem(registrationKey) }
  }

  SideEffect {
    paneSelectionState?.registerItem(
      key = registrationKey,
      enabled = enabled,
      onSelected = { latestOnSelected.value() },
    )
  }

  Box(
    modifier =
      Modifier.fillMaxWidth()
        .onGloballyPositioned { coordinates ->
          paneSelectionState?.updateItemLayoutCoordinates(registrationKey, coordinates)
        }
        .hoverFeedback(
          interactionSource,
          enabled,
          AppShapes.rounded(PopoverDefaults.ExpandedRadius - PopoverDefaults.PanePadding),
        )
        .clickable(enabled = enabled, interactionSource = interactionSource) {
          if (paneSelectionState?.consumeSuppressedClick(registrationKey) == true) {
            return@clickable
          }
          latestOnSelected.value()
        }
  ) {
    content()
  }
}
