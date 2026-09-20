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
  }
}
