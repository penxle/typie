public func resolveActiveSiteId(stored: String?, available: [String]) -> String? {
  if let stored, available.contains(stored) { return stored }
  return available.first
}
