package co.typie.ui.component.popover

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppTheme

sealed interface PopoverMenuEntry

internal data object PopoverMenuDividerEntry : PopoverMenuEntry

internal data class PopoverMenuItemEntry(
  val content: @Composable () -> Unit,
  val onClick: () -> Unit,
) : PopoverMenuEntry

internal data class PopoverMenuStaticEntry(val content: @Composable () -> Unit) : PopoverMenuEntry

class PopoverMenuScope {
  @PublishedApi internal val entries = mutableListOf<PopoverMenuEntry>()

  fun item(icon: IconData, label: String, color: Color? = null, onClick: () -> Unit) {
    entries.add(
      PopoverMenuItemEntry(
        content = { PopoverMenuItemRow(icon = icon, label = label, color = color) },
        onClick = onClick,
      )
    )
  }

  fun item(content: @Composable () -> Unit, onClick: () -> Unit) {
    entries.add(PopoverMenuItemEntry(content = content, onClick = onClick))
  }

  fun divider() {
    entries.add(PopoverMenuDividerEntry)
  }

  fun static(content: @Composable () -> Unit) {
    entries.add(PopoverMenuStaticEntry(content = content))
  }
}

@Composable
internal fun PopoverMenuItemRow(icon: IconData, label: String, color: Color? = null) {
  val resolvedColor = color ?: AppTheme.colors.textDefault

  Row(
    modifier = Modifier.height(42.dp).padding(horizontal = 16.dp),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Icon(icon = icon, modifier = Modifier.size(18.dp), tint = resolvedColor)
    Spacer(Modifier.width(12.dp))
    Text(text = label, style = AppTheme.typography.action, color = resolvedColor)
  }
}

@Composable
fun PopoverMenu(
  anchor: @Composable () -> Unit,
  enabled: Boolean = true,
  placement: PopoverPlacement = PopoverPlacement.BelowEnd,
  maxWidth: Dp? = null,
  minWidth: Dp = 0.dp,
  screenPadding: PaddingValues = PaddingValues(all = PopoverDefaults.ScreenPadding),
  collapsedCornerRadius: Dp? = null,
  content: PopoverMenuScope.() -> Unit,
) {
  Popover(
    anchor = anchor,
    enabled = enabled,
    placement = placement,
    maxWidth = maxWidth,
    minWidth = minWidth,
    screenPadding = screenPadding,
    collapsedCornerRadius = collapsedCornerRadius,
    pane = {
      val entries = PopoverMenuScope().apply(content).entries

      Column(modifier = Modifier.padding(PopoverDefaults.PanePadding)) {
        val segments = segmentEntries(entries)
        segments.forEachIndexed { index, segment ->
          if (index > 0 && segment is MenuSegment.Divider) {
            Spacer(Modifier.height(12.dp))
            Box(
              Modifier.fillMaxWidth()
                .height(1.dp)
                .padding(horizontal = 8.dp)
                .background(AppTheme.colors.borderHairline)
            )
            Spacer(Modifier.height(12.dp))
          }
          when (segment) {
            is MenuSegment.Items -> {
              PopoverList(
                items =
                  segment.entries.map { entry ->
                    PopoverListItem(
                      content = entry.content,
                      onSelected = {
                        close()
                        entry.onClick()
                      },
                    )
                  }
              )
            }
            is MenuSegment.Static -> {
              segment.content()
            }
            is MenuSegment.Divider -> {}
          }
        }
      }
    },
  )
}

private sealed interface MenuSegment {
  data class Items(val entries: List<PopoverMenuItemEntry>) : MenuSegment

  data class Static(val content: @Composable () -> Unit) : MenuSegment

  data object Divider : MenuSegment
}

private fun segmentEntries(entries: List<PopoverMenuEntry>): List<MenuSegment> {
  val segments = mutableListOf<MenuSegment>()
  val currentItems = mutableListOf<PopoverMenuItemEntry>()

  fun flushItems() {
    if (currentItems.isNotEmpty()) {
      segments.add(MenuSegment.Items(currentItems.toList()))
      currentItems.clear()
    }
  }

  for (entry in entries) {
    when (entry) {
      is PopoverMenuItemEntry -> currentItems.add(entry)
      is PopoverMenuDividerEntry -> {
        flushItems()
        segments.add(MenuSegment.Divider)
      }
      is PopoverMenuStaticEntry -> {
        flushItems()
        segments.add(MenuSegment.Static(entry.content))
      }
    }
  }

  flushItems()
  return segments
}
