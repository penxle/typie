package co.typie.ui.component.topbar

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import co.typie.navigation.LocalRoute

enum class NavDirection { Push, Pop, Switch }

@Stable
class TopBarState {
  internal val leadingEntries = mutableStateMapOf<Any, @Composable () -> Unit>()
  internal val centerEntries = mutableStateMapOf<Any, @Composable () -> Unit>()
  internal val trailingEntries = mutableStateMapOf<Any, @Composable () -> Unit>()
  internal val customEntries = mutableStateMapOf<Any, @Composable () -> Unit>()
  var customKey: Any by mutableStateOf(NullKey)

  fun setCustom(key: Any, content: (@Composable () -> Unit)?) {
    if (content != null) {
      customEntries[key] = content
    } else {
      customEntries.remove(key)
    }
    customKey = if (content != null) key else NullKey
  }

  var leadingKey: Any by mutableStateOf(NullKey)
  var centerKey: Any by mutableStateOf(NullKey)
  var trailingKey: Any by mutableStateOf(NullKey)

  var scrollOffset: (() -> Int)? by mutableStateOf(null)
  var visible: Boolean by mutableStateOf(true)
  var enabled: Boolean by mutableStateOf(false)
  var navDirection: NavDirection by mutableStateOf(NavDirection.Switch)
  /** 0f = blur 없음, 1f = full blur. NavigationStack이 전환 progress에 연동하여 업데이트. */
  var blurFactor: Float by mutableStateOf(1f)

  fun setLeading(key: Any, content: (@Composable () -> Unit)?) {
    if (content != null) leadingEntries[key] = content else leadingEntries.remove(key)
    leadingKey = key
  }

  fun setCenter(key: Any, content: (@Composable () -> Unit)?) {
    if (content != null) centerEntries[key] = content else centerEntries.remove(key)
    centerKey = key
  }

  fun setTrailing(key: Any, content: (@Composable () -> Unit)?) {
    if (content != null) trailingEntries[key] = content else trailingEntries.remove(key)
    trailingKey = key
  }

  fun reset() {
    visible = false
    scrollOffset = null
  }

  fun clearRoute(key: Any) {
    leadingEntries.remove(key)
    centerEntries.remove(key)
    trailingEntries.remove(key)
    customEntries.remove(key)
  }

  companion object {
    val DefaultLeadingKey = Any()
    internal val NullKey = Any()
    internal val NormalModeKey = Any()
  }
}

val LocalTopBarState = staticCompositionLocalOf<TopBarState?> { null }

@Composable
fun ProvideTopBar(
  enabled: Boolean = true,
  leading: (@Composable () -> Unit)? = { TopBarBackButton() },
  leadingKey: Any = TopBarState.DefaultLeadingKey,
  center: (@Composable () -> Unit)? = null,
  centerKey: Any = LocalRoute.current,
  trailing: (@Composable () -> Unit)? = null,
  trailingKey: Any = LocalRoute.current,
  scrollOffset: (() -> Int)? = null,
  visible: Boolean = true,
  custom: (@Composable () -> Unit)? = null,
  customKey: Any = LocalRoute.current,
) {
  val state = LocalTopBarState.current ?: return
  state.enabled = enabled
  if (enabled) {
    state.setLeading(if (leading != null) leadingKey else TopBarState.NullKey, leading)
    state.setCenter(if (center != null) centerKey else TopBarState.NullKey, center)
    state.setTrailing(if (trailing != null) trailingKey else TopBarState.NullKey, trailing)
    state.scrollOffset = scrollOffset
    state.visible = visible
    state.setCustom(customKey, custom)
  } else {
    state.setLeading(TopBarState.NullKey, null)
    state.setCenter(TopBarState.NullKey, null)
    state.setTrailing(TopBarState.NullKey, null)
    state.scrollOffset = null
    state.setCustom(customKey, null)
  }
}
