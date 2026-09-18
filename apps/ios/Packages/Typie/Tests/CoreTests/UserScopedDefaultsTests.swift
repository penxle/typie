import Foundation
import Testing

@testable import Core

@Suite struct UserScopedDefaultsTests {
  @Test func writesSiteIdUnderScopedKey() {
    withFreshDefaults { defaults in
      let preferences = UserScopedDefaults(defaults: defaults)
      preferences.switchUser("A")
      preferences.siteId = "site-a"
      #expect(preferences.siteId == "site-a")
      #expect(defaults.string(forKey: "site_id@A") == "site-a")
      #expect(defaults.object(forKey: "site_id") == nil)
    }
  }

  @Test func keepsValuesSeparatePerUser() {
    withFreshDefaults { defaults in
      let preferences = UserScopedDefaults(defaults: defaults)
      preferences.switchUser("A")
      preferences.siteId = "site-a"

      preferences.switchUser("B")
      #expect(preferences.siteId == nil)
      preferences.siteId = "site-b"
      #expect(defaults.string(forKey: "site_id@B") == "site-b")

      preferences.switchUser("A")
      #expect(preferences.siteId == "site-a")
      #expect(defaults.string(forKey: "site_id@A") == "site-a")
    }
  }

  @Test func readsValuePersistedByAnotherInstance() {
    withFreshDefaults { defaults in
      defaults.set("site-a", forKey: "site_id@A")
      let preferences = UserScopedDefaults(defaults: defaults)
      preferences.switchUser("A")
      #expect(preferences.siteId == "site-a")
    }
  }

  @Test func clearsValueWhenNoUserIsBound() {
    withFreshDefaults { defaults in
      let preferences = UserScopedDefaults(defaults: defaults)
      preferences.switchUser("A")
      preferences.siteId = "site-a"
      preferences.switchUser(nil)
      #expect(preferences.siteId == nil)
      #expect(defaults.string(forKey: "site_id@A") == "site-a")
    }
  }

  @Test func dropsWritesWhileNoUserIsBound() {
    withFreshDefaults { defaults in
      let preferences = UserScopedDefaults(defaults: defaults)
      preferences.switchUser(nil)
      preferences.siteId = "site-x"
      #expect(defaults.object(forKey: "site_id") == nil)
      preferences.switchUser("A")
      #expect(preferences.siteId == nil)
      #expect(defaults.object(forKey: "site_id@A") == nil)
    }
  }

  @Test func keepsUnpersistedWriteInMemoryWhileNoUserIsBound() {
    withFreshDefaults { defaults in
      let preferences = UserScopedDefaults(defaults: defaults)
      preferences.switchUser(nil)
      preferences.siteId = "site-x"
      #expect(preferences.siteId == "site-x")
      #expect(defaults.object(forKey: "site_id") == nil)
    }
  }

  @Test func migratesUnscopedValueOnFirstSwitch() {
    withFreshDefaults { defaults in
      defaults.set("site-legacy", forKey: "site_id")
      let preferences = UserScopedDefaults(defaults: defaults)
      preferences.switchUser("A")
      #expect(preferences.siteId == "site-legacy")
      #expect(defaults.string(forKey: "site_id@A") == "site-legacy")
      #expect(defaults.object(forKey: "site_id") == nil)
    }
  }

  @Test func migratesOnlyOnce() {
    withFreshDefaults { defaults in
      defaults.set("site-legacy", forKey: "site_id")
      let preferences = UserScopedDefaults(defaults: defaults)
      preferences.switchUser("A")
      preferences.switchUser("B")
      #expect(preferences.siteId == nil)
      #expect(defaults.object(forKey: "site_id@B") == nil)
      #expect(defaults.string(forKey: "site_id@A") == "site-legacy")
    }
  }

  @Test func keepsExistingScopedValueAndDropsUnscopedOnMigration() {
    withFreshDefaults { defaults in
      defaults.set("site-legacy", forKey: "site_id")
      defaults.set("site-a", forKey: "site_id@A")
      let preferences = UserScopedDefaults(defaults: defaults)
      preferences.switchUser("A")
      #expect(preferences.siteId == "site-a")
      #expect(defaults.string(forKey: "site_id@A") == "site-a")
      #expect(defaults.object(forKey: "site_id") == nil)
    }
  }

  @Test func removesScopedKeyWhenValueIsClearedWhileBound() {
    withFreshDefaults { defaults in
      let preferences = UserScopedDefaults(defaults: defaults)
      preferences.switchUser("A")
      preferences.siteId = "site-a"
      preferences.siteId = nil
      #expect(preferences.siteId == nil)
      #expect(defaults.object(forKey: "site_id@A") == nil)
    }
  }
}
