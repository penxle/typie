package co.typie.dev

enum class DevToolsAccent {
  Muted,
  Success,
  Warning,
  Danger,
  Info,
  Highlight,
}

fun NetworkPreset.devToolsAccent(): DevToolsAccent =
  when (this) {
    NetworkPreset.Normal -> DevToolsAccent.Success
    NetworkPreset.Slow -> DevToolsAccent.Warning
    NetworkPreset.Offline -> DevToolsAccent.Danger
  }

fun devToolsCollapsedIndicatorAccents(
  networkPreset: NetworkPreset,
  hardwareKeyboardConnected: Boolean = false,
): List<DevToolsAccent> {
  return listOf(
    networkPreset.devToolsAccent(),
    if (hardwareKeyboardConnected) DevToolsAccent.Info else DevToolsAccent.Muted,
  )
}
