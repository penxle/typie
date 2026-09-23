import Core
import FactoryKit
import Foundation
import Observation

@MainActor @Observable
final class HomeLayoutStore {
  private(set) var order: [HomeSection]
  private(set) var hidden: Set<HomeSection>
  private(set) var recentSort: RecentSort

  @ObservationIgnored private let preferences: UserPreferences

  init() {
    let preferences = Container.shared.userPreferences()
    self.preferences = preferences
    order = Self.sanitize(preferences.homeSectionOrder())
    hidden = Set(preferences.hiddenHomeSections().compactMap(HomeSection.init(rawValue:)))
      .filter(\.canHide)
    recentSort = preferences.recentDocumentsSort().flatMap(RecentSort.init(rawValue:)) ?? .viewed
  }

  func setRecentSort(_ sort: RecentSort) {
    guard sort != recentSort else { return }
    recentSort = sort
    preferences.setRecentDocumentsSort(sort.rawValue)
  }

  var visible: [HomeSection] { order.filter { !hidden.contains($0) } }

  func isVisible(_ section: HomeSection) -> Bool {
    !hidden.contains(section)
  }

  func setVisible(_ section: HomeSection, _ visible: Bool) {
    guard section.canHide, isVisible(section) != visible else { return }
    if visible {
      hidden.remove(section)
    } else {
      hidden.insert(section)
    }
    preferences.setHiddenHomeSections(HomeSection.allCases.filter(hidden.contains).map(\.rawValue))
  }

  func setOrder(_ sections: [HomeSection]) {
    let next = Self.sanitize(sections.map(\.rawValue))
    guard next != order else { return }
    order = next
    preferences.setHomeSectionOrder(order.map(\.rawValue))
  }

  static func sanitize(_ ids: [String]) -> [HomeSection] {
    var order: [HomeSection] = []
    for section in ids.compactMap(HomeSection.init(rawValue:)) where !order.contains(section) {
      order.append(section)
    }
    for section in HomeSection.allCases where !order.contains(section) {
      order.append(section)
    }
    return order
  }
}
