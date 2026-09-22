import Foundation

enum EntityText {
  static func documentTitle(_ title: String) -> String {
    title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? "(제목 없음)" : title
  }

  static func folderName(_ name: String) -> String {
    name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? "(이름 없음)" : name
  }

  static func excerpt(_ text: String) -> String {
    text.isEmpty ? "(내용 없음)" : text
  }

  static func folderSummary(folders: Int, documents: Int) -> String {
    if folders == 0, documents == 0 { return "빈 폴더" }
    if folders == 0 { return "문서 \(Formatting.comma(documents))개" }
    return "폴더 \(Formatting.comma(folders))개 · 문서 \(Formatting.comma(documents))개"
  }

  static func spaceSummary(folders: Int, documents: Int) -> String {
    folderCounts(folders: folders, documents: documents) ?? "비어 있는 스페이스"
  }

  static func folderMetadataSummary(folders: Int, documents: Int, characters: Int) -> String {
    (countParts(folders: folders, documents: documents) + ["총 \(Formatting.comma(characters))자"])
      .joined(separator: " · ")
  }

  static func folderCounts(folders: Int, documents: Int) -> String? {
    let parts = countParts(folders: folders, documents: documents)
    return parts.isEmpty ? nil : parts.joined(separator: " · ")
  }

  private static func countParts(folders: Int, documents: Int) -> [String] {
    var parts: [String] = []
    if folders > 0 { parts.append("폴더 \(Formatting.comma(folders))개") }
    if documents > 0 { parts.append("문서 \(Formatting.comma(documents))개") }
    return parts
  }
}
