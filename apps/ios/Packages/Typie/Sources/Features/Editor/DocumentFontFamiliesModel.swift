import Core
import Editor
import FactoryKit
import Foundation
import GraphQL
import Observation

@MainActor @Observable
final class DocumentFontFamiliesModel {
  private typealias Family = DocumentScreen_FontFamilies_Query.Data.Me.DocumentFontFamily

  private(set) var families: [EditorFontFamily]?
  private(set) var loadFailed = false

  @ObservationIgnored private let query: WatchQuery<NoInput, DocumentScreen_FontFamilies_Query>

  init() {
    query = WatchQuery(
      client: Container.shared.graphQLClient(), query: DocumentScreen_FontFamilies_Query())
    keepObserving(while: self) { [weak self] in self?.sync() }
  }

  private func sync() {
    let error = query.error
    if let me = query.data?.me {
      let next = me.documentFontFamilies.compactMap(Self.family)
      if next != families {
        families = next
      }
      loadFailed = false
    }
    if error != nil, families == nil {
      loadFailed = true
    }
  }

  private static func family(_ family: Family) -> EditorFontFamily? {
    let source: EditorFontFamily.Source
    switch family.source {
    case .case(.default): source = .default
    case .case(.user): source = .user
    case .case(.fallback): source = .fallback
    case .unknown: return nil
    }
    return EditorFontFamily(
      name: family.familyName, source: source,
      fonts: family.fonts.compactMap { font in
        guard let weight = UInt16(exactly: font.weight), let url = URL(string: font.url),
          !font.hash.isEmpty
        else { return nil }
        return EditorFontFamily.Font(weight: weight, url: url, hash: font.hash)
      })
  }
}
