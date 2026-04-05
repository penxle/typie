package co.typie.ui.component.topbar

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
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

internal fun needsImplicitRouteKey(
  enabled: Boolean,
  center: (@Composable () -> Unit)?,
  centerKey: Any?,
  trailing: (@Composable () -> Unit)?,
  trailingKey: Any?,
  custom: (@Composable () -> Unit)?,
  customKey: Any?,
): Boolean {
  if (!enabled) return false

  return (center != null && centerKey == null) ||
    (trailing != null && trailingKey == null) ||
    (custom != null && customKey == null)
}

internal fun resolveTopBarEntryKey(
  explicitKey: Any?,
  routeKey: Any?,
  fallbackKey: Any,
): Any {
  return explicitKey ?: routeKey ?: fallbackKey
}

@Composable
fun ProvideTopBar(
  enabled: Boolean = true,
  leading: (@Composable () -> Unit)? = { TopBarBackButton() },
  leadingKey: Any = TopBarState.DefaultLeadingKey,
  center: (@Composable () -> Unit)? = null,
  centerKey: Any? = null,
  trailing: (@Composable () -> Unit)? = null,
  trailingKey: Any? = null,
  scrollOffset: (() -> Int)? = null,
  visible: Boolean = true,
  custom: (@Composable () -> Unit)? = null,
  customKey: Any? = null,
) {
  val state = LocalTopBarState.current ?: return
  val fallbackEntryKey = remember { Any() }

  state.enabled = enabled
  if (enabled) {
    val routeKey = if (needsImplicitRouteKey(enabled, center, centerKey, trailing, trailingKey, custom, customKey)) {
      LocalRoute.current
    } else {
      null
    }

    state.setLeading(if (leading != null) leadingKey else TopBarState.NullKey, leading)
    state.setCenter(
      if (center != null) resolveTopBarEntryKey(centerKey, routeKey, fallbackEntryKey) else TopBarState.NullKey,
      center,
    )
    state.setTrailing(
      if (trailing != null) resolveTopBarEntryKey(trailingKey, routeKey, fallbackEntryKey) else TopBarState.NullKey,
      trailing,
    )
    state.scrollOffset = scrollOffset
    state.visible = visible
    state.setCustom(
      resolveTopBarEntryKey(customKey, routeKey, fallbackEntryKey),
      custom,
    )
  } else {
    state.setLeading(TopBarState.NullKey, null)
    state.setCenter(TopBarState.NullKey, null)
    state.setTrailing(TopBarState.NullKey, null)
    state.scrollOffset = null
    state.setCustom(TopBarState.NullKey, null)
  }
}
