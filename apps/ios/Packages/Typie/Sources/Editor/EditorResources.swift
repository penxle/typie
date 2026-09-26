internal import EditorFFI
import Foundation

@MainActor
protocol ResourceReceiver: AnyObject {
  func receiveResourceUpdate(_ update: RResourceUpdate) throws
}

@MainActor
public final class EditorResources {
  private let host: REditorHost
  private var receivers: [Registration] = []
  private var latest: RResourceUpdate?

  private struct Registration {
    weak var receiver: (any ResourceReceiver)?
  }

  private init(host: REditorHost) {
    self.host = host
  }

  public static func load() async throws -> EditorResources {
    let host = try await Task.detached(priority: .userInitiated) {
      try REditorHost(icuData: EditorICU.load())
    }.value
    return EditorResources(host: host)
  }

  nonisolated func createEditor(doc: String, viewport: String) throws -> REditor {
    try host.createEditorFromDoc(doc: doc, viewport: viewport)
  }

  public func setTheme(isDark: Bool) throws {
    let variant = try EditorJSON.encode(Self.themeVariant(isDark: isDark))
    try commit { host in try host.setThemeVariant(variant: variant) }
  }

  static func themeVariant(isDark: Bool) -> ThemeVariant {
    isDark ? .darkBlack : .lightWhite
  }

  func commit(_ change: (REditorHost) throws -> RResourceUpdate?) rethrows {
    guard let update = try change(host) else { return }
    latest = update
    var live: [Registration] = []
    for registration in receivers {
      guard let receiver = registration.receiver,
        (try? receiver.receiveResourceUpdate(update)) != nil
      else { continue }
      live.append(registration)
    }
    receivers = live
  }

  func register(_ receiver: any ResourceReceiver) throws {
    receivers.removeAll { $0.receiver == nil }
    guard !receivers.contains(where: { $0.receiver === receiver }) else { return }
    if let latest {
      try receiver.receiveResourceUpdate(latest)
    }
    receivers.append(Registration(receiver: receiver))
  }
}

extension EditorResources: FontCommitTarget {
  func setFonts(_ families: [FontFamily]) throws {
    let json = try families.map { try EditorJSON.encode($0) }
    try commit { host in try host.setFonts(families: json) }
  }

  func addFont(_ part: FontData, family: String, weight: UInt16, data: Data) throws {
    try commit { host in
      switch part {
      case .manifest:
        try host.addFontManifest(family: family, weight: weight, data: data)
      case .base:
        try host.addFontBase(family: family, weight: weight, data: data)
      case .chunk(let id):
        try host.addFontChunk(family: family, weight: weight, chunkId: id, data: data)
      }
    }
  }
}
