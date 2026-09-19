import Foundation
import Testing

@testable import Core

@Suite struct UserPreferencesTests {
  @Test func writesSiteIdUnderScopedKey() {
    withFreshDefaults { defaults in
      let preferences = UserPreferences(userId: "A", defaults: defaults)
      preferences.siteId = "site-a"
      #expect(preferences.siteId == "site-a")
      #expect(defaults.string(forKey: "site_id@A") == "site-a")
      #expect(defaults.object(forKey: "site_id") == nil)
    }
  }

  @Test func keepsSiteIdSeparatePerUser() {
    withFreshDefaults { defaults in
      let a = UserPreferences(userId: "A", defaults: defaults)
      a.siteId = "site-a"

      let b = UserPreferences(userId: "B", defaults: defaults)
      #expect(b.siteId == nil)
      b.siteId = "site-b"
      #expect(defaults.string(forKey: "site_id@B") == "site-b")

      #expect(a.siteId == "site-a")
      #expect(defaults.string(forKey: "site_id@A") == "site-a")
    }
  }

  @Test func readsSiteIdPersistedByAnotherInstance() {
    withFreshDefaults { defaults in
      defaults.set("site-a", forKey: "site_id@A")
      #expect(UserPreferences(userId: "A", defaults: defaults).siteId == "site-a")
    }
  }

  @Test func removesScopedKeyWhenSiteIdIsCleared() {
    withFreshDefaults { defaults in
      let preferences = UserPreferences(userId: "A", defaults: defaults)
      preferences.siteId = "site-a"
      preferences.siteId = nil
      #expect(preferences.siteId == nil)
      #expect(defaults.object(forKey: "site_id@A") == nil)
    }
  }

  @Test func storesRecentSearchesUnderScopedKey() {
    withFreshDefaults { defaults in
      let preferences = UserPreferences(userId: "A", defaults: defaults)
      preferences.recentSearches = ["x", "y"]
      #expect(preferences.recentSearches == ["x", "y"])
      #expect(defaults.stringArray(forKey: "recent_searches@A") == ["x", "y"])
      #expect(defaults.object(forKey: "recent_searches") == nil)
    }
  }

  @Test func keepsRecentSearchesSeparatePerUser() {
    withFreshDefaults { defaults in
      UserPreferences(userId: "A", defaults: defaults).recentSearches = ["x", "y"]
      #expect(UserPreferences(userId: "B", defaults: defaults).recentSearches == [])
      #expect(UserPreferences(userId: "A", defaults: defaults).recentSearches == ["x", "y"])
    }
  }

  @Test func emptyRecentSearchesRemovesKey() {
    withFreshDefaults { defaults in
      let preferences = UserPreferences(userId: "A", defaults: defaults)
      preferences.recentSearches = ["x"]
      preferences.recentSearches = []
      #expect(preferences.recentSearches == [])
      #expect(defaults.object(forKey: "recent_searches@A") == nil)
    }
  }
}
