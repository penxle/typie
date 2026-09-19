import Testing

@testable import Core

@MainActor @Suite struct ActiveSiteStoreTests {
  @Test func startsFromTheStoredSiteId() {
    withFreshDefaults { defaults in
      defaults.set("site-1", forKey: "site_id@user-1")
      let preferences = UserPreferences(userId: "user-1", defaults: defaults)
      #expect(ActiveSiteStore(preferences: preferences).siteId == "site-1")
    }
  }

  @Test func startsWithoutSiteIdWhenNothingIsStored() {
    withFreshDefaults { defaults in
      let preferences = UserPreferences(userId: "user-1", defaults: defaults)
      #expect(ActiveSiteStore(preferences: preferences).siteId == nil)
    }
  }

  @Test func selectPersistsAndPublishes() {
    withFreshDefaults { defaults in
      let preferences = UserPreferences(userId: "user-1", defaults: defaults)
      let store = ActiveSiteStore(preferences: preferences)
      store.select("site-2")
      #expect(store.siteId == "site-2")
      #expect(preferences.siteId == "site-2")
      #expect(defaults.string(forKey: "site_id@user-1") == "site-2")
    }
  }

  @Test func keepsStoredSiteIdWhenAvailable() {
    #expect(ActiveSiteStore.resolve(stored: "S2", available: ["S1", "S2"]) == "S2")
  }

  @Test func fallsBackToFirstSiteWhenStoredIsInvalid() {
    #expect(ActiveSiteStore.resolve(stored: "GONE", available: ["S1", "S2"]) == "S1")
  }

  @Test func fallsBackToFirstSiteWhenStoredIsNil() {
    #expect(ActiveSiteStore.resolve(stored: nil, available: ["S1", "S2"]) == "S1")
  }

  @Test func returnsNilWhenNoSitesAvailable() {
    #expect(ActiveSiteStore.resolve(stored: "S1", available: []) == nil)
    #expect(ActiveSiteStore.resolve(stored: nil, available: []) == nil)
  }
}
