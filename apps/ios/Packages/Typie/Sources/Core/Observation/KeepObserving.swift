import Observation

@MainActor
public func keepObserving<Owner: AnyObject & Sendable>(
  while owner: Owner, _ apply: @escaping @MainActor @Sendable () -> Void
) {
  withObservationTracking(apply) { [weak owner] in
    Task { @MainActor in
      guard let owner else { return }
      keepObserving(while: owner, apply)
    }
  }
}
