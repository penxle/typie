import Apollo
import Foundation
import Synchronization

private final class OneShot: Sendable {
  private let claimed = Mutex(false)

  var isClaimed: Bool { claimed.withLock { $0 } }

  func claim() -> Bool {
    claimed.withLock { value in
      if value { return false }
      value = true
      return true
    }
  }
}

struct SessionCookieInterceptor: HTTPInterceptor {
  static let cookieName = "typie-st"

  let onSessionCookie: @Sendable (String) async throws -> Void

  func intercept(
    request: URLRequest, next: NextHTTPInterceptorFunction
  ) async throws -> HTTPResponse {
    let handler = onSessionCookie
    let once = OneShot()
    return try await next(request).mapChunks { response, chunk in
      guard !once.isClaimed, let value = Self.sessionCookie(in: response), once.claim() else {
        return chunk
      }
      do {
        try await handler(value)
      } catch let error as CancellationError {
        throw error
      } catch {
        throw SessionEstablishmentError(underlying: error)
      }
      return chunk
    }
  }

  static func sessionCookie(in response: HTTPURLResponse) -> String? {
    guard let url = response.url, let header = response.value(forHTTPHeaderField: "Set-Cookie")
    else { return nil }
    return HTTPCookie.cookies(withResponseHeaderFields: ["Set-Cookie": header], for: url)
      .first { $0.name == cookieName }?.value
  }
}
