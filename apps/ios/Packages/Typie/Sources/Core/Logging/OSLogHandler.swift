import Logging
import os

public struct OSLogHandler: LogHandler {
  public var logLevel: Logging.Logger.Level = .info
  public var metadata: Logging.Logger.Metadata = [:]
  public var metadataProvider: Logging.Logger.MetadataProvider?

  private let logger: os.Logger

  public init(label: String) {
    let destination = Self.destination(for: label)
    logger = os.Logger(subsystem: destination.subsystem, category: destination.category)
  }

  public subscript(metadataKey key: String) -> Logging.Logger.Metadata.Value? {
    get { metadata[key] }
    set { metadata[key] = newValue }
  }

  public func log(event: LogEvent) {
    let message = event.message.description
    let type = Self.type(for: event.level)
    if let details = details(of: event) {
      logger.log(level: type, "\(message, privacy: .public) \(details, privacy: .private)")
    } else {
      logger.log(level: type, "\(message, privacy: .public)")
    }
  }

  func details(of event: LogEvent) -> String? {
    var merged = metadata
    if let provided = metadataProvider?.get() {
      merged.merge(provided) { _, new in new }
    }
    if let extra = event.metadata {
      merged.merge(extra) { _, new in new }
    }
    if let error = event.error {
      merged["error"] = "\(error)"
    }
    return merged.isEmpty ? nil : Self.render(merged)
  }

  static func destination(for label: String) -> (subsystem: String, category: String) {
    guard let dot = label.lastIndex(of: ".") else { return (label, "default") }
    return (String(label[..<dot]), String(label[label.index(after: dot)...]))
  }

  static func type(for level: Logging.Logger.Level) -> OSLogType {
    switch level {
    case .trace, .debug: .debug
    case .info: .info
    case .notice, .warning: .default
    case .error: .error
    case .critical: .fault
    }
  }

  static func render(_ metadata: Logging.Logger.Metadata) -> String {
    metadata.sorted { $0.key < $1.key }.map { "\($0.key)=\($0.value)" }.joined(separator: " ")
  }
}
