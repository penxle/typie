import Testing

@testable import Features

@Suite struct EntityCardListTests {
  private func document(_ id: String) -> EntityContainerItem {
    .entity(
      .document(
        EntityDocumentItem(
          entityId: id, icon: EntityIconSpec(kind: .document, name: "", color: ""), path: [],
          title: id, subtitle: nil, excerpt: "", updatedAt: nil)))
  }

  private func folder(_ id: String) -> EntityContainerItem {
    .entity(
      .folder(
        EntityFolderItem(
          entityId: id, icon: EntityIconSpec(kind: .folder, name: "", color: ""), path: [],
          title: id, folderCount: 1, documentCount: 2)))
  }

  private func doc(_ id: String) -> EntityDocumentItem {
    guard case .entity(.document(let document)) = document(id) else { fatalError() }
    return document
  }

  @Test func emptyAndSingleDocument() {
    #expect(EntityCardListRules.blocks([]) == [])
    #expect(EntityCardListRules.blocks([document("a")]) == [.documents([doc("a")])])
  }

  @Test func consecutiveDocumentsShareOneCard() {
    let blocks = EntityCardListRules.blocks([document("a"), document("b"), document("c")])
    #expect(blocks == [.documents([doc("a"), doc("b"), doc("c")])])
    #expect(blocks.map(\.id) == ["documents-a"])
  }

  @Test func foldersAndDividersSplitRunsInOrder() {
    let items = [document("a"), folder("f1"), document("b"), .divider(id: "x1"), document("c")]
    guard case .entity(.folder(let f1)) = folder("f1") else { return }
    #expect(
      EntityCardListRules.blocks(items) == [
        .documents([doc("a")]), .folder(f1), .documents([doc("b")]), .divider(id: "x1"),
        .documents([doc("c")]),
      ])
  }

  @Test func leadingTrailingAndConsecutiveDividersAreKept() {
    let items: [EntityContainerItem] = [
      .divider(id: "x0"), document("a"), .divider(id: "x1"), .divider(id: "x2"), document("b"),
      .divider(id: "x3"),
    ]
    let blocks = EntityCardListRules.blocks(items)
    #expect(
      blocks == [
        .divider(id: "x0"), .documents([doc("a")]), .divider(id: "x1"), .divider(id: "x2"),
        .documents([doc("b")]), .divider(id: "x3"),
      ])
    #expect(Set(blocks.map(\.id)).count == 6)
  }
}
