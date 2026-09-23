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

  @Test func storesExpandedFoldersPerSite() {
    withFreshDefaults { defaults in
      let preferences = UserPreferences(userId: "A", defaults: defaults)
      preferences.setExpandedFolders(["f2", "f1"], siteId: "site-1")
      #expect(preferences.expandedFolders(siteId: "site-1") == ["f2", "f1"])
      #expect(preferences.expandedFolders(siteId: "site-2") == [])
      #expect(defaults.stringArray(forKey: "expanded_folders:site-1@A") == ["f2", "f1"])
    }
  }

  @Test func storesHomeLayoutUnderScopedKeys() {
    withFreshDefaults { defaults in
      let preferences = UserPreferences(userId: "A", defaults: defaults)
      preferences.setHomeSectionOrder(["all", "goal"])
      preferences.setHiddenHomeSections(["recent"])
      #expect(defaults.stringArray(forKey: "home_section_order@A") == ["all", "goal"])
      #expect(defaults.stringArray(forKey: "home_hidden_sections@A") == ["recent"])
      let reloaded = UserPreferences(userId: "A", defaults: defaults)
      #expect(reloaded.homeSectionOrder() == ["all", "goal"])
      #expect(reloaded.hiddenHomeSections() == ["recent"])
      reloaded.setHiddenHomeSections([])
      #expect(defaults.object(forKey: "home_hidden_sections@A") == nil)
      reloaded.setRecentDocumentsSort("updated")
      #expect(defaults.string(forKey: "recent_documents_sort@A") == "updated")
      #expect(UserPreferences(userId: "A", defaults: defaults).recentDocumentsSort() == "updated")
      #expect(UserPreferences(userId: "B", defaults: defaults).homeSectionOrder() == [])
    }
  }

  @Test func clearingExpandedFoldersRemovesKey() {
    withFreshDefaults { defaults in
      let preferences = UserPreferences(userId: "A", defaults: defaults)
      preferences.setExpandedFolders(["f1"], siteId: "site-1")
      preferences.setExpandedFolders([], siteId: "site-1")
      #expect(defaults.object(forKey: "expanded_folders:site-1@A") == nil)
      #expect(
        UserPreferences(userId: "A", defaults: defaults).expandedFolders(siteId: "site-1") == [])
    }
  }
}
