import Alamofire
import Foundation

enum HTTPSession {
  static func configuration() -> URLSessionConfiguration {
    let configuration = URLSessionConfiguration.ephemeral
    configuration.httpCookieStorage = nil
    configuration.httpShouldSetCookies = false
    configuration.httpCookieAcceptPolicy = .never
    configuration.timeoutIntervalForRequest = 60
    return configuration
  }

  static func make(
    configuration: URLSessionConfiguration = HTTPSession.configuration()
  ) -> Session {
    Session(configuration: configuration, redirectHandler: Redirector(behavior: .doNotFollow))
  }
}
