import Core
import FactoryKit
import FactoryTesting
import Foundation
import Testing

@testable import Features

@MainActor
@Suite(.container) struct HomeLayoutStoreTests {
  private func makeStore(order: [String] = [], hidden: [String] = []) -> (
    HomeLayoutStore, TestPreferences
  ) {
    let preferences = makeTestPreferences(siteId: "site-1")
    preferences.userPreferences.setHomeSectionOrder(order)
    preferences.userPreferences.setHiddenHomeSections(hidden)
    Container.shared.userPreferences.register { preferences.userPreferences }
    return (Container.shared.homeLayoutStore(), preferences)
  }

  @Test func defaultsToAllSectionsInDeclaredOrder() {
    let (store, _) = makeStore()
    #expect(store.order == [.goal, .pinned, .recent, .all])
    #expect(store.visible == [.goal, .pinned, .recent, .all])
  }

  @Test func restoresOrderAndHiddenSections() {
    let (store, _) = makeStore(order: ["all", "recent", "goal", "pinned"], hidden: ["goal"])
    #expect(store.order == [.all, .recent, .goal, .pinned])
    #expect(store.visible == [.all, .recent, .pinned])
  }

  @Test func repairsCorruptedOrderAndIgnoresHiddenAll() {
    let (store, _) = makeStore(order: ["recent", "bogus", "recent"], hidden: ["all", "bogus"])
    #expect(store.order == [.recent, .goal, .pinned, .all])
    #expect(store.hidden.isEmpty)
  }

  @Test func togglingVisibilityPersistsAndRefusesAll() {
    let (store, preferences) = makeStore()
    store.setVisible(.recent, false)
    store.setVisible(.all, false)
    #expect(store.visible == [.goal, .pinned, .all])
    #expect(preferences.userPreferences.hiddenHomeSections() == ["recent"])
    store.setVisible(.recent, true)
    #expect(preferences.userPreferences.hiddenHomeSections() == [])
  }

  @Test func recentSortDefaultsToViewedAndPersists() {
    let (store, preferences) = makeStore()
    #expect(store.recentSort == .viewed)
    store.setRecentSort(.updated)
    #expect(preferences.userPreferences.recentDocumentsSort() == "updated")
    preferences.userPreferences.setRecentDocumentsSort("bogus")
    Container.shared.userPreferences.register { preferences.userPreferences }
    #expect(HomeLayoutStore().recentSort == .viewed)
  }

  @Test func settingOrderPersistsAndSanitizes() {
    let (store, preferences) = makeStore()
    store.setOrder([.all, .goal, .pinned, .recent])
    #expect(store.order == [.all, .goal, .pinned, .recent])
    #expect(preferences.userPreferences.homeSectionOrder() == ["all", "goal", "pinned", "recent"])
    store.setOrder([.recent, .recent])
    #expect(store.order == [.recent, .goal, .pinned, .all])
  }
}
