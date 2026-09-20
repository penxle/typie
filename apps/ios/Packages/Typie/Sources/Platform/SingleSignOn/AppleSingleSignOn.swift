#if canImport(UIKit)

  import AuthenticationServices
  import Core
  import UIKit

  @MainActor
  final class AppleSingleSignOn: NSObject, SingleSignOnAdapter {
    private let anchor: @MainActor () -> UIWindow?
    private var controller: ASAuthorizationController?
    private var continuation: CheckedContinuation<String, any Error>?

    init(anchor: @escaping @MainActor () -> UIWindow?) {
      self.anchor = anchor
    }

    func authenticate() async throws -> SingleSignOnCredential {
      guard anchor() != nil else { throw SingleSignOnError.noPresenter }
      let code: String = try await withCheckedThrowingContinuation { continuation in
        self.continuation = continuation
        let request = ASAuthorizationAppleIDProvider().createRequest()
        request.requestedScopes = [.email]
        let controller = ASAuthorizationController(authorizationRequests: [request])
        controller.delegate = self
        controller.presentationContextProvider = self
        self.controller = controller
        controller.performRequests()
      }
      return SingleSignOnCredential(provider: .apple, params: ["code": code])
    }

    private func finish(_ result: Result<String, any Error>) {
      continuation?.resume(with: result)
      continuation = nil
      controller = nil
    }
  }

  extension AppleSingleSignOn: ASAuthorizationControllerDelegate {
    func authorizationController(
      controller: ASAuthorizationController,
      didCompleteWithAuthorization authorization: ASAuthorization
    ) {
      guard let credential = authorization.credential as? ASAuthorizationAppleIDCredential,
        let data = credential.authorizationCode, let code = String(data: data, encoding: .utf8)
      else {
        finish(.failure(SingleSignOnError.missingCredential))
        return
      }
      finish(.success(code))
    }

    func authorizationController(
      controller: ASAuthorizationController, didCompleteWithError error: any Error
    ) {
      if let error = error as? ASAuthorizationError, error.code == .canceled {
        finish(.failure(SingleSignOnError.cancelled))
      } else {
        finish(.failure(error))
      }
    }
  }

  extension AppleSingleSignOn: ASAuthorizationControllerPresentationContextProviding {
    func presentationAnchor(for controller: ASAuthorizationController) -> ASPresentationAnchor {
      anchor()!
    }
  }

#endif
