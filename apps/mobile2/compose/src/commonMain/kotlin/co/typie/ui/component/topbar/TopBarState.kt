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

enum class NavDirection {
  Push,
  Pop,
  Switch,
}

@Stable
class TopBarState {
  internal val leadingEntries = mutableStateMapOf<Any, @Composable () -> Unit>()
  internal val centerEntries = mutableStateMapOf<Any, @Composable () -> Unit>()
  internal val trailingEntries = mutableStateMapOf<Any, @Composable () -> Unit>()
  internal val customEntries = mutableStateMapOf<Any, @Composable () -> Unit>()
  internal val leadingOwners = mutableMapOf<Any, Any>()
  internal val centerOwners = mutableMapOf<Any, Any>()
  internal val trailingOwners = mutableMapOf<Any, Any>()
  internal val customOwners = mutableMapOf<Any, Any>()
  internal val leadingInstances = mutableStateMapOf<Any, Any>()
  internal val centerInstances = mutableStateMapOf<Any, Any>()
  internal val trailingInstances = mutableStateMapOf<Any, Any>()
  internal val customInstances = mutableStateMapOf<Any, Any>()
  var customKey: Any by mutableStateOf(NullKey)

  fun setCustom(
    key: Any,
    content: (@Composable () -> Unit)?,
    owner: Any? = null,
    instance: Any? = null,
  ) {
    setEntry(customEntries, customOwners, customInstances, key, content, owner, instance)
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
  var animatedVisibilityAlpha: Float by mutableStateOf(0f)
  var animatedVisibilityOffsetY: Float by mutableStateOf(0f)

  fun setLeading(
    key: Any,
    content: (@Composable () -> Unit)?,
    owner: Any? = null,
    instance: Any? = null,
  ) {
    setEntry(leadingEntries, leadingOwners, leadingInstances, key, content, owner, instance)
    leadingKey = key
  }

  fun setCenter(
    key: Any,
    content: (@Composable () -> Unit)?,
    owner: Any? = null,
    instance: Any? = null,
  ) {
    setEntry(centerEntries, centerOwners, centerInstances, key, content, owner, instance)
    centerKey = key
  }

  fun setTrailing(
    key: Any,
    content: (@Composable () -> Unit)?,
    owner: Any? = null,
    instance: Any? = null,
  ) {
    setEntry(trailingEntries, trailingOwners, trailingInstances, key, content, owner, instance)
    trailingKey = key
  }

  fun reset() {
    visible = false
    scrollOffset = null
  }

  fun clearRoute(key: Any) {
    clearOwnedEntries(
      route = key,
      currentKey = leadingKey,
      entries = leadingEntries,
      owners = leadingOwners,
      instances = leadingInstances,
      onCurrentKeyCleared = { leadingKey = it },
    )
    clearOwnedEntries(
      route = key,
      currentKey = centerKey,
      entries = centerEntries,
      owners = centerOwners,
      instances = centerInstances,
      onCurrentKeyCleared = { centerKey = it },
    )
    clearOwnedEntries(
      route = key,
      currentKey = trailingKey,
      entries = trailingEntries,
      owners = trailingOwners,
      instances = trailingInstances,
      onCurrentKeyCleared = { trailingKey = it },
    )
    clearOwnedEntries(
      route = key,
      currentKey = customKey,
      entries = customEntries,
      owners = customOwners,
      instances = customInstances,
      onCurrentKeyCleared = { customKey = it },
    )
  }

  private fun setEntry(
    entries: MutableMap<Any, @Composable () -> Unit>,
    owners: MutableMap<Any, Any>,
    instances: MutableMap<Any, Any>,
    key: Any,
    content: (@Composable () -> Unit)?,
    owner: Any?,
    instance: Any?,
  ) {
    if (content != null) {
      entries[key] = content
      if (key != NullKey && owner != null) {
        owners[key] = owner
      } else {
        owners.remove(key)
      }
      if (key != NullKey && instance != null) {
        instances[key] = instance
      } else {
        instances.remove(key)
      }
    } else {
      entries.remove(key)
      owners.remove(key)
      instances.remove(key)
    }
  }

  private fun clearOwnedEntries(
    route: Any,
    currentKey: Any,
    entries: MutableMap<Any, @Composable () -> Unit>,
    owners: MutableMap<Any, Any>,
    instances: MutableMap<Any, Any>,
    onCurrentKeyCleared: (Any) -> Unit,
  ) {
    val keysToRemove = linkedSetOf(route)
    owners.entries
      .filterTo(mutableListOf()) { (_, owner) -> owner == route }
      .forEach { (entryKey, _) -> keysToRemove += entryKey }

    keysToRemove.forEach { entryKey ->
      entries.remove(entryKey)
      owners.remove(entryKey)
      instances.remove(entryKey)
    }

    if (currentKey in keysToRemove) {
      onCurrentKeyCleared(NullKey)
    }
  }

  companion object {
    val DefaultLeadingKey = Any()
    internal val NullKey = Any()
    internal val NormalModeKey = Any()
  }
}

val LocalTopBarState = staticCompositionLocalOf<TopBarState?> { null }
val LocalTopBarAnimationSource = staticCompositionLocalOf<TopBarState?> { null }

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

internal fun resolveTopBarEntryKey(explicitKey: Any?, routeKey: Any?, fallbackKey: Any): Any {
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
  val owner = LocalRoute.current

  state.enabled = enabled
  if (enabled) {
    val entryInstance = remember { Any() }
    val routeKey =
      if (
        needsImplicitRouteKey(enabled, center, centerKey, trailing, trailingKey, custom, customKey)
      ) {
        owner
      } else {
        null
      }
    val resolvedLeadingKey = if (leading != null) leadingKey else TopBarState.NullKey
    val resolvedCenterKey =
      if (center != null) resolveTopBarEntryKey(centerKey, routeKey, fallbackEntryKey)
      else TopBarState.NullKey
    val resolvedTrailingKey =
      if (trailing != null) resolveTopBarEntryKey(trailingKey, routeKey, fallbackEntryKey)
      else TopBarState.NullKey
    val resolvedCustomKey = resolveTopBarEntryKey(customKey, routeKey, fallbackEntryKey)

    state.setLeading(resolvedLeadingKey, leading, owner, entryInstance)
    state.setCenter(resolvedCenterKey, center, owner, entryInstance)
    state.setTrailing(resolvedTrailingKey, trailing, owner, entryInstance)
    state.scrollOffset = scrollOffset
    state.visible = visible
    state.setCustom(resolvedCustomKey, custom, owner, entryInstance)
  } else {
    state.setLeading(TopBarState.NullKey, null)
    state.setCenter(TopBarState.NullKey, null)
    state.setTrailing(TopBarState.NullKey, null)
    state.scrollOffset = null
    state.setCustom(TopBarState.NullKey, null)
  }
}
