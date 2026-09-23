import Testing

@testable import Features

@Suite struct EntityTextTests {
  @Test func blankTitlesAndNames() {
    #expect(EntityText.documentTitle("  ") == "(제목 없음)")
    #expect(EntityText.documentTitle("t") == "t")
    #expect(EntityText.folderName("") == "(이름 없음)")
    #expect(EntityText.excerpt("") == "(내용 없음)")
    #expect(EntityText.excerpt(" ") == " ")
  }

  @Test func folderSummary() {
    #expect(EntityText.folderSummary(folders: 0, documents: 0) == "빈 폴더")
    #expect(EntityText.folderSummary(folders: 0, documents: 3) == "문서 3개")
    #expect(EntityText.folderSummary(folders: 2, documents: 1234) == "폴더 2개 · 문서 1,234개")
    #expect(EntityText.folderSummary(folders: 2, documents: 0) == "폴더 2개 · 문서 0개")
  }

  @Test func spaceSummary() {
    #expect(EntityText.spaceSummary(folders: 0, documents: 0) == "비어 있는 스페이스")
    #expect(EntityText.spaceSummary(folders: 3, documents: 0) == "폴더 3개")
    #expect(EntityText.spaceSummary(folders: 0, documents: 12) == "문서 12개")
    #expect(EntityText.spaceSummary(folders: 1, documents: 2345) == "폴더 1개 · 문서 2,345개")
  }

  @Test func folderMetadataSummary() {
    #expect(
      EntityText.folderMetadataSummary(folders: 0, documents: 0, characters: 0) == "총 0자")
    #expect(
      EntityText.folderMetadataSummary(folders: 0, documents: 4, characters: 1500)
        == "문서 4개 · 총 1,500자")
    #expect(
      EntityText.folderMetadataSummary(folders: 2, documents: 4, characters: 12345)
        == "폴더 2개 · 문서 4개 · 총 12,345자")
  }

  @Test func folderCounts() {
    #expect(EntityText.folderCounts(folders: 0, documents: 0) == nil)
    #expect(EntityText.folderCounts(folders: 2, documents: 0) == "폴더 2개")
    #expect(EntityText.folderCounts(folders: 2, documents: 1) == "폴더 2개 · 문서 1개")
  }
}
