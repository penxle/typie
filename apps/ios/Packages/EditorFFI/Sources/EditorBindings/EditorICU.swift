import Foundation

public enum EditorICU {
  public static func load() throws -> Data {
    guard let url = Bundle.module.url(forResource: "icu", withExtension: "zst") else {
      throw CocoaError(.fileNoSuchFile)
    }
    return try Data(contentsOf: url)
  }
}
