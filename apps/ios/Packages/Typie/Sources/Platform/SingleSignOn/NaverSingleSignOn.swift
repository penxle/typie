#if canImport(UIKit)

  import Core
  import NidThirdPartyLogin

  @MainActor
  final class NaverSingleSignOn: SingleSignOnAdapter {
    func authenticate() async throws -> SingleSignOnCredential {
      let accessToken: String = try await withCheckedThrowingContinuation { continuation in
        NidOAuth.shared.requestLogin { result in
          switch result {
          case .success(let login):
            continuation.resume(returning: login.accessToken.tokenString)
          case .failure(let error):
            continuation.resume(throwing: Self.mapped(error))
          }
        }
      }
      return SingleSignOnCredential(provider: .naver, params: ["access_token": accessToken])
    }

    private static func mapped(_ error: NidError) -> any Error {
      if case .clientError(.canceledByUser) = error {
        return SingleSignOnError.cancelled
      }
      return error
    }
  }

#endif
