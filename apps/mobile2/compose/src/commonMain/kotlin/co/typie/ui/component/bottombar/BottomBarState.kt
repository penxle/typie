package co.typie.ui.component.bottombar

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import co.typie.navigation.LocalRoute

@Stable
class BottomBarState {
  internal val pillEntries = mutableStateMapOf<Any, @Composable () -> Unit>()
  internal val actionEntries = mutableStateMapOf<Any, @Composable () -> Unit>()
  internal val customEntries = mutableStateMapOf<Any, @Composable () -> Unit>()

  var pillKey: Any by mutableStateOf(NullKey)
  var actionKey: Any by mutableStateOf(NullKey)
  var customKey: Any by mutableStateOf(NullKey)

  var enabled: Boolean by mutableStateOf(false)

  fun setPill(key: Any, content: (@Composable () -> Unit)?) {
    if (content != null) pillEntries[key] = content else pillEntries.remove(key)
    pillKey = key
  }

  fun setAction(key: Any, content: (@Composable () -> Unit)?) {
    if (content != null) actionEntries[key] = content else actionEntries.remove(key)
    actionKey = key
  }

  fun setCustom(key: Any, content: (@Composable () -> Unit)?) {
    if (content != null) customEntries[key] = content else customEntries.remove(key)
    customKey = if (content != null) key else NullKey
  }

  fun clearRoute(key: Any) {
    pillEntries.remove(key)
    actionEntries.remove(key)
    customEntries.remove(key)
  }

  companion object {
    internal val NullKey = Any()
  }
}

val LocalBottomBarState = staticCompositionLocalOf<BottomBarState?> { null }

private fun needsImplicitRouteKey(
  enabled: Boolean,
  pill: (@Composable () -> Unit)?,
  pillKey: Any?,
  action: (@Composable () -> Unit)?,
  actionKey: Any?,
  custom: (@Composable () -> Unit)?,
  customKey: Any?,
): Boolean {
  if (!enabled) return false
  return (pill != null && pillKey == null) ||
    (action != null && actionKey == null) ||
    (custom != null && customKey == null)
}

private fun resolveBottomBarEntryKey(explicitKey: Any?, routeKey: Any?, fallbackKey: Any): Any {
  return explicitKey ?: routeKey ?: fallbackKey
}

@Composable
fun ProvideBottomBar(
  enabled: Boolean = true,
  pill: (@Composable () -> Unit)? = null,
  pillKey: Any? = null,
  action: (@Composable () -> Unit)? = null,
  actionKey: Any? = null,
  custom: (@Composable () -> Unit)? = null,
  customKey: Any? = null,
) {
  val state = LocalBottomBarState.current ?: return
  val fallbackEntryKey = remember { Any() }

  state.enabled = enabled
  if (enabled) {
    val routeKey =
      if (needsImplicitRouteKey(enabled, pill, pillKey, action, actionKey, custom, customKey)) {
        LocalRoute.current
      } else {
        null
      }

    state.setPill(
      if (pill != null) resolveBottomBarEntryKey(pillKey, routeKey, fallbackEntryKey)
      else BottomBarState.NullKey,
      pill,
    )
    state.setAction(
      if (action != null) resolveBottomBarEntryKey(actionKey, routeKey, fallbackEntryKey)
      else BottomBarState.NullKey,
      action,
    )
    state.setCustom(resolveBottomBarEntryKey(customKey, routeKey, fallbackEntryKey), custom)
  }
}
