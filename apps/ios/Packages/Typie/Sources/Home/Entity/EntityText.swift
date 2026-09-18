import Foundation

public enum EntityText {
  public static func documentTitle(_ title: String) -> String {
    title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? "(제목 없음)" : title
  }

  public static func folderName(_ name: String) -> String {
    name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? "(이름 없음)" : name
  }

  public static func excerpt(_ text: String) -> String {
    text.isEmpty ? "(내용 없음)" : text
  }

  public static func folderSummary(folders: Int, documents: Int) -> String {
    if folders == 0, documents == 0 { return "빈 폴더" }
    if folders == 0 { return "문서 \(comma(documents))개" }
    return "폴더 \(comma(folders))개 · 문서 \(comma(documents))개"
  }

  private static func comma(_ value: Int) -> String {
    value.formatted(.number.grouping(.automatic).locale(Locale(identifier: "en_US")))
  }
}
