import Testing

@testable import Core

@MainActor @Suite struct ActiveSiteStoreTests {
  @Test func selectPersistsAndPublishes() {
    withFreshDefaults { defaults in
      let preferences = UserScopedDefaults(defaults: defaults)
      preferences.switchUser("user-1")
      let store = ActiveSiteStore(preferences: preferences)
      store.select("site-2")
      #expect(store.siteId == "site-2")
      #expect(preferences.siteId == "site-2")
    }
  }

  @Test func publishUpdatesStateOnly() async {
    await withFreshDefaultsAsync { defaults in
      let preferences = UserScopedDefaults(defaults: defaults)
      let store = ActiveSiteStore(preferences: preferences)
      await store.publish("site-1")
      #expect(store.siteId == "site-1")
      #expect(preferences.siteId == nil)
      await store.publish(nil)
      #expect(store.siteId == nil)
    }
  }
}
