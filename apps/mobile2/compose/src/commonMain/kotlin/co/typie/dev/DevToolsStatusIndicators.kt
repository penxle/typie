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

fun SubscriptionDevScenario.devToolsAccent(): DevToolsAccent =
  when (this) {
    SubscriptionDevScenario.RemoteData -> DevToolsAccent.Muted
    SubscriptionDevScenario.NoSubscription,
    SubscriptionDevScenario.TrialExpired -> DevToolsAccent.Muted
    SubscriptionDevScenario.Trial -> DevToolsAccent.Info
    SubscriptionDevScenario.Monthly,
    SubscriptionDevScenario.Yearly -> DevToolsAccent.Success

    SubscriptionDevScenario.CancelScheduled -> DevToolsAccent.Warning
    SubscriptionDevScenario.BillingKey,
    SubscriptionDevScenario.Manual -> DevToolsAccent.Highlight
  }

fun devToolsCollapsedIndicatorAccents(
  networkPreset: NetworkPreset,
  subscriptionScenario: SubscriptionDevScenario,
): List<DevToolsAccent> {
  return listOf(networkPreset.devToolsAccent(), subscriptionScenario.devToolsAccent())
}
