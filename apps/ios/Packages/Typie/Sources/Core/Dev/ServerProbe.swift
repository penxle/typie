import Alamofire
import Foundation

public struct ServerProbe: Sendable {
  private let config: AppConfig
  private let client: GraphQLClient
  private let session: Session
  public let deviceID: String

  init(config: AppConfig, client: GraphQLClient, session: Session, deviceID: String) {
    self.config = config
    self.client = client
    self.session = session
    self.deviceID = deviceID
  }

  public var apiHost: String { config.apiURL.host() ?? "?" }
  public var authHost: String { config.authURL.host() ?? "?" }

  public func graphQL() async -> String {
    do {
      let response = try await client.apollo.fetch(
        query: ServerProbe_Query(), cachePolicy: .networkOnly)
      if let errors = response.errors, !errors.isEmpty {
        return "errors: \(errors.compactMap(\.message).joined(separator: "; "))"
      }
      guard let data = response.data else { return "no data" }
      return "randomName=\(data.randomName) me=\(data.me == nil ? "null" : "present")"
    } catch {
      return "failed: \(error.localizedDescription)"
    }
  }

  public func authorize() async -> String {
    let response = await session.request(
      config.authURL.appending(path: "authorize"),
      parameters: OIDCClient.authorizeParameters(config: config)
    ).serializingData(emptyResponseCodes: OIDCClient.emptyResponseCodes).response
    guard let http = response.response else {
      return "failed: \(response.error?.localizedDescription ?? "unknown")"
    }
    let location = http.value(forHTTPHeaderField: "Location")
    let error = OIDCClient.redirectQueryItem("error", in: location)
    return
      "status=\(http.statusCode) location=\(location == nil ? "absent" : "present") error=\(error ?? "nil")"
  }
}
