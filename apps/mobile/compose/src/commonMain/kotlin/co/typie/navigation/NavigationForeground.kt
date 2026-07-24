package co.typie.navigation

import androidx.compose.foundation.layout.Box
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalContext
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.currentCompositionLocalContext
import androidx.compose.runtime.getValue
import androidx.compose.runtime.movableContentOf
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.PointerEvent
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.node.ModifierNodeElement
import androidx.compose.ui.node.PointerInputModifierNode
import androidx.compose.ui.unit.IntSize
import androidx.lifecycle.ViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.LocalViewModelStoreOwner
import co.typie.route.Route
import co.typie.ui.component.bottombar.BottomBarState
import co.typie.ui.component.bottombar.LocalBottomBarState
import co.typie.ui.component.topbar.LocalTopBarState
import co.typie.ui.component.topbar.TopBarState

internal class NavigationForegroundRegistry {
  private val entries = mutableStateListOf<NavigationForegroundEntry>()
  private val backdropEntries = mutableStateListOf<NavigationTopBarBackdropEntry>()

  val topBarBackdropBackground: Color?
    get() = backdropEntries.lastOrNull()?.background

  fun register(entry: NavigationForegroundEntry) {
    if (entries.none { it.owner === entry.owner }) {
      entries += entry
    }
  }

  fun unregister(entry: NavigationForegroundEntry) {
    entries.removeAll { it.owner === entry.owner }
  }

  fun register(entry: NavigationTopBarBackdropEntry) {
    if (backdropEntries.none { it.owner === entry.owner }) {
      backdropEntries += entry
    }
  }

  fun unregister(entry: NavigationTopBarBackdropEntry) {
    backdropEntries.removeAll { it.owner === entry.owner }
  }

  @Composable
  fun Content(
    route: Route,
    viewModelStoreOwner: ViewModelStoreOwner,
    topBarState: TopBarState?,
    bottomBarState: BottomBarState?,
    modifier: Modifier,
  ) {
    entries.forEach { entry ->
      Box(
        modifier =
          modifier.then(
            if (entry.sharePointerInputWithSiblings) {
              ShareNavigationForegroundPointerInputElement
            } else {
              Modifier
            }
          )
      ) {
        CompositionLocalProvider(entry.context) {
          CompositionLocalProvider(
            LocalViewModelStoreOwner provides viewModelStoreOwner,
            LocalRoute provides route,
            LocalTopBarState provides topBarState,
            LocalBottomBarState provides bottomBarState,
          ) {
            entry.content()
          }
        }
      }
    }
  }
}

internal class NavigationForegroundEntry(
  val owner: Any,
  val content: @Composable () -> Unit,
  context: CompositionLocalContext,
  sharePointerInputWithSiblings: Boolean,
) {
  var context by mutableStateOf(context)
  var sharePointerInputWithSiblings by mutableStateOf(sharePointerInputWithSiblings)
}

internal class NavigationTopBarBackdropEntry(val owner: Any, background: Color) {
  var background by mutableStateOf(background)
}

private val LocalNavigationForegroundRegistry =
  staticCompositionLocalOf<NavigationForegroundRegistry?> { null }

@Composable
internal fun NavigationForeground(
  sharePointerInputWithSiblings: Boolean = false,
  content: @Composable () -> Unit,
) {
  val registry = LocalNavigationForegroundRegistry.current
  if (registry == null) {
    content()
    return
  }

  val currentContent = rememberUpdatedState(content)
  val movableContent = remember { movableContentOf { currentContent.value() } }
  val context = currentCompositionLocalContext
  val entry =
    remember(registry) {
      NavigationForegroundEntry(
        owner = Any(),
        content = movableContent,
        context = context,
        sharePointerInputWithSiblings = sharePointerInputWithSiblings,
      )
    }
  if (entry.context != context) {
    entry.context = context
  }
  entry.sharePointerInputWithSiblings = sharePointerInputWithSiblings
  registry.register(entry)

  DisposableEffect(registry, entry) { onDispose { registry.unregister(entry) } }
}

@Composable
internal fun PublishNavigationTopBarBackdropStyle(background: Color) {
  val registry = LocalNavigationForegroundRegistry.current ?: return
  val entry =
    remember(registry) { NavigationTopBarBackdropEntry(owner = Any(), background = background) }
  entry.background = background
  registry.register(entry)

  DisposableEffect(registry, entry) { onDispose { registry.unregister(entry) } }
}

@Composable
internal fun ProvideNavigationForegroundRegistry(
  registry: NavigationForegroundRegistry,
  content: @Composable () -> Unit,
) {
  CompositionLocalProvider(LocalNavigationForegroundRegistry provides registry, content = content)
}

private data object ShareNavigationForegroundPointerInputElement :
  ModifierNodeElement<ShareNavigationForegroundPointerInputNode>() {
  override fun create(): ShareNavigationForegroundPointerInputNode =
    ShareNavigationForegroundPointerInputNode()

  override fun update(node: ShareNavigationForegroundPointerInputNode) = Unit
}

private class ShareNavigationForegroundPointerInputNode :
  Modifier.Node(), PointerInputModifierNode {
  override fun sharePointerInputWithSiblings(): Boolean = true

  override fun onPointerEvent(pointerEvent: PointerEvent, pass: PointerEventPass, bounds: IntSize) =
    Unit

  override fun onCancelPointerInput() = Unit
}
