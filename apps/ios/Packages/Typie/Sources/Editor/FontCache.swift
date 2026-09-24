import CryptoKit
import Foundation

struct FontCache: Sendable {
  let directory: URL

  func read(_ url: URL) async -> Data? {
    try? Data(contentsOf: file(for: url))
  }

  func write(_ data: Data, for url: URL) async {
    try? FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    try? data.write(to: file(for: url), options: .atomic)
  }

  func remove(_ url: URL) async {
    try? FileManager.default.removeItem(at: file(for: url))
  }

  func file(for url: URL) -> URL {
    let digest = SHA256.hash(data: Data(url.absoluteString.utf8))
    return directory.appending(path: digest.map { String(format: "%02x", $0) }.joined())
  }
}
