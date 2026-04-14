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

fun devToolsCollapsedIndicatorAccents(networkPreset: NetworkPreset): List<DevToolsAccent> {
  return listOf(networkPreset.devToolsAccent())
}
