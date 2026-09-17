import Core
import Design
import Observation

@Observable
@MainActor
public final class SingleSignOnModel {
  public private(set) var activeProvider: SingleSignOnProvider?

  @ObservationIgnored private let login: @Sendable (SingleSignOnProvider) async throws -> Void
  @ObservationIgnored private let toast: TToastCenter
  @ObservationIgnored private let onSuccess: @MainActor () -> Void

  public init(
    login: @escaping @Sendable (SingleSignOnProvider) async throws -> Void,
    toast: TToastCenter,
    onSuccess: @escaping @MainActor () -> Void
  ) {
    self.login = login
    self.toast = toast
    self.onSuccess = onSuccess
  }

  public var isBusy: Bool { activeProvider != nil }

  public func signIn(with provider: SingleSignOnProvider) async {
    guard activeProvider == nil else { return }
    activeProvider = provider
    defer { activeProvider = nil }
    do {
      try await login(provider)
      onSuccess()
    } catch SingleSignOnError.cancelled {
      return
    } catch is CancellationError {
      return
    } catch {
      toast.error("오류가 발생했어요. 잠시 후 다시 시도해주세요.")
    }
  }
}
