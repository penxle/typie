public struct EditorDebugOverlays: OptionSet, Sendable {
  public let rawValue: Int

  public init(rawValue: Int) {
    self.rawValue = rawValue
  }

  public static let viewportGuides = EditorDebugOverlays(rawValue: 1 << 0)
  public static let bodyAreas = EditorDebugOverlays(rawValue: 1 << 1)
  public static let pageSurfaces = EditorDebugOverlays(rawValue: 1 << 2)
}
