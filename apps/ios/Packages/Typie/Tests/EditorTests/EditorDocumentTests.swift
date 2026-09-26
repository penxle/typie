import EditorFFI
import Foundation
import Testing

@testable import Editor

@MainActor @Suite final class EditorDocumentTests {
  private let cacheDirectory = FileManager.default.temporaryDirectory.appending(
    path: "document-\(UUID().uuidString)")

  deinit {
    try? FileManager.default.removeItem(at: cacheDirectory)
  }

  @Test(arguments: EditorSyntheticLayout.allCases)
  func syntheticDocumentsSpellOutTheirRootModifiers(layout: EditorSyntheticLayout) {
    let root = EditorDocument.synthetic(layout).plain.root

    #expect(
      root.node == .root(PlainRootNode(layoutMode: EditorDocument.syntheticLayoutMode(layout))))
    #expect(
      root.modifiers == [
        .fontFamily: .fontFamily(value: "Pretendard"),
        .fontSize: .fontSize(value: 1200),
        .fontWeight: .fontWeight(value: 400),
        .textColor: .textColor(value: "black"),
        .backgroundColor: .backgroundColor(value: "none"),
        .letterSpacing: .letterSpacing(value: 0),
        .lineHeight: .lineHeight(value: 160),
        .paragraphIndent: .paragraphIndent(value: 100),
        .blockGap: .blockGap(value: 100),
      ])
    #expect(root.children.count == 3002)
    #expect(root.children.allSatisfy { $0.node == .paragraph && $0.children.count == 1 })
  }

  @Test func syntheticLayoutsMatchTheMeasurementDocument() {
    #expect(
      EditorDocument.syntheticLayoutMode(.paginated)
        == .paginated(
          pageWidth: 794, pageHeight: 1123, pageMarginTop: 94, pageMarginBottom: 94,
          pageMarginLeft: 94, pageMarginRight: 94))
    #expect(EditorDocument.syntheticLayoutMode(.continuous) == .continuous(maxWidth: 600))
  }

  @Test(arguments: zip(EditorSyntheticLayout.allCases, [215, 267]))
  func syntheticDocumentsDrawWithTheFixtureFontAndFillTheMeasuredPages(
    layout: EditorSyntheticLayout, pages: Int
  ) async throws {
    let resources = try await EditorResources.load()
    try await preloadFixtureFont(resources, family: "Pretendard", cacheDirectory: cacheDirectory)
    let fonts = FontLoader(
      target: resources, fetch: FakeFontNetwork().fetch, sleep: { _ in },
      cacheDirectory: cacheDirectory)
    let session = try await EditorSession.open(
      .synthetic(layout), resources: resources, fonts: fonts, viewport: testViewport)
    let update = try session.frame(frameRequest())

    #expect(update.geometry.pageCount == UInt32(pages))
    #expect(update.geometry.tick == nil)
    #expect(setTiles(update).contains { hasInk($0.image) })
  }
}
