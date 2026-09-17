import Core
import GoogleSignIn
import UIKit

@MainActor
final class GoogleSingleSignOn: SingleSignOnAdapter {
  private let presenter: @MainActor () -> UIViewController?

  init(presenter: @escaping @MainActor () -> UIViewController?) {
    self.presenter = presenter
  }

  func authenticate() async throws -> SingleSignOnCredential {
    guard let presenter = presenter() else { throw SingleSignOnError.noPresenter }
    let result: GIDSignInResult
    do {
      result = try await GIDSignIn.sharedInstance.signIn(withPresenting: presenter)
    } catch let error as GIDSignInError where error.code == .canceled {
      throw SingleSignOnError.cancelled
    }
    guard let code = result.serverAuthCode else { throw SingleSignOnError.missingCredential }
    return SingleSignOnCredential(provider: .google, params: ["code": code])
  }
}
