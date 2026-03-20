import AuthenticationServices
import Foundation
import UIKit

@MainActor @objcMembers public class AppleSingleSignOnBridge : NSObject {
  private var delegate: AuthorizationDelegate?

  public func authenticate(completion: @escaping @Sendable (String?, NSError?) -> Void) {
    let provider = ASAuthorizationAppleIDProvider()
    let request = provider.createRequest()
    request.requestedScopes = [.email]

    let controller = ASAuthorizationController(authorizationRequests: [request])

    let delegate = AuthorizationDelegate(completion: completion)
    self.delegate = delegate

    controller.delegate = delegate
    controller.presentationContextProvider = delegate
    controller.performRequests()
  }
}

private class AuthorizationDelegate : NSObject, ASAuthorizationControllerDelegate,
  ASAuthorizationControllerPresentationContextProviding
{
  private let completion: @Sendable (String?, NSError?) -> Void

  init(completion: @escaping @Sendable (String?, NSError?) -> Void) {
    self.completion = completion
  }

  func authorizationController(
    controller: ASAuthorizationController,
    didCompleteWithAuthorization authorization: ASAuthorization
  ) {
    guard
      let credential = authorization.credential as? ASAuthorizationAppleIDCredential,
      let codeData = credential.authorizationCode,
      let code = String(data: codeData, encoding: .utf8)
    else {
      completion(nil, nil)
      return
    }
    
    completion(code, nil)
  }

  func authorizationController(
    controller: ASAuthorizationController,
    didCompleteWithError error: any Error
  ) {
    completion(nil, error as NSError)
  }

  @MainActor
  func presentationAnchor(for controller: ASAuthorizationController) -> ASPresentationAnchor {
    UIApplication.shared.activeWindow ?? UIWindow()
  }
}
