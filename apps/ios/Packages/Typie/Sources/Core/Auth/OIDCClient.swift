import Alamofire
import Foundation

struct Me: Sendable, Equatable {
  let id: String
  let siteIds: [String]
}

struct OIDCClient: Sendable {
  static let emptyResponseCodes: Set<Int> = [204, 205, 302]

  private static let redirectURI = "typie:///authorize"
  private static let logoutRedirectURI = "typie:///"
  private static let meQuery = "query AuthService_Me { me { id sites { id } } }"

  private let config: AppConfig
  private let session: Session

  init(config: AppConfig, session: Session) {
    self.config = config
    self.session = session
  }

  func exchange(sessionToken: String) async throws -> String {
    try await token(code: authorize(sessionToken: sessionToken))
  }

  func logout(sessionToken: String) async {
    _ = await session.request(
      config.authURL.appending(path: "logout"),
      parameters: ["redirect_uri": Self.logoutRedirectURI],
      headers: Self.cookieHeaders(sessionToken: sessionToken)
    ).serializingData(emptyResponseCodes: Self.emptyResponseCodes).response
  }

  func fetchMe(accessToken: String) async throws -> Me {
    let response = await session.request(
      config.apiURL.appending(path: "graphql"),
      method: .post,
      parameters: MeRequest(query: Self.meQuery),
      encoder: JSONParameterEncoder.default,
      headers: HTTPHeaders([.authorization(bearerToken: accessToken)])
    ).validate().serializingData().response

    let http = try Self.httpResponse(response, path: "/graphql me")
    guard (200..<300).contains(http.statusCode) else { throw HTTPError.status(http.statusCode) }
    guard let data = response.data,
      let body = try? JSONDecoder().decode(MeResponse.self, from: data)
    else {
      throw HTTPError.malformedResponse("/graphql me: no data")
    }
    guard let me = body.data?.me else {
      throw HTTPError.malformedResponse("/graphql me: \(body.errors?.first?.message ?? "no data")")
    }
    return Me(id: me.id, siteIds: me.sites.map(\.id))
  }

  static func authorizeParameters(config: AppConfig) -> [String: String] {
    [
      "response_type": "code",
      "redirect_uri": redirectURI,
      "client_id": config.oidcClientID,
      "prompt": "none",
    ]
  }

  static func redirectQueryItem(_ name: String, in location: String?) -> String? {
    location.flatMap(URLComponents.init(string:))?.queryItems?.first { $0.name == name }?.value
  }

  private func authorize(sessionToken: String) async throws -> String {
    let response = await session.request(
      config.authURL.appending(path: "authorize"),
      parameters: Self.authorizeParameters(config: config),
      headers: Self.cookieHeaders(sessionToken: sessionToken)
    ).serializingData(emptyResponseCodes: Self.emptyResponseCodes).response

    let http = try Self.httpResponse(response, path: "/authorize")
    guard http.statusCode == 302 else { throw HTTPError.status(http.statusCode) }
    guard let location = http.value(forHTTPHeaderField: "Location") else {
      throw HTTPError.malformedResponse("/authorize: no Location header in redirect response")
    }

    if let error = Self.redirectQueryItem("error", in: location) {
      guard error == "login_required" else {
        throw HTTPError.malformedResponse("/authorize: \(error)")
      }
      throw InvalidCredentialsError()
    }
    guard let code = Self.redirectQueryItem("code", in: location) else {
      throw HTTPError.malformedResponse("/authorize: no code in redirect response")
    }
    return code
  }

  private func token(code: String) async throws -> String {
    let response = await session.request(
      config.authURL.appending(path: "token"),
      method: .post,
      parameters: [
        "code": code,
        "grant_type": "authorization_code",
        "redirect_uri": Self.redirectURI,
        "client_id": config.oidcClientID,
        "client_secret": config.oidcClientSecret,
      ],
      encoder: URLEncodedFormParameterEncoder.default
    ).validate().serializingData().response

    let http = try Self.httpResponse(response, path: "/token")
    guard (200..<300).contains(http.statusCode) else {
      if (400..<500).contains(http.statusCode), let data = response.data,
        let failure = try? JSONDecoder().decode(TokenError.self, from: data),
        failure.error == "invalid_grant"
      {
        throw InvalidCredentialsError()
      }
      throw HTTPError.status(http.statusCode)
    }
    guard let data = response.data,
      let body = try? JSONDecoder().decode(TokenResponse.self, from: data)
    else {
      throw HTTPError.malformedResponse("/token: no access_token in response")
    }
    return body.accessToken
  }

  private static func cookieHeaders(sessionToken: String) -> HTTPHeaders {
    HTTPHeaders([HTTPHeader(name: "Cookie", value: "typie-st=\(sessionToken)")])
  }

  private static func httpResponse(
    _ response: DataResponse<Data, AFError>, path: String
  ) throws -> HTTPURLResponse {
    try Task.checkCancellation()
    if let error = response.error {
      if error.isExplicitlyCancelledError
        || (error.underlyingError as? URLError)?.code == .cancelled
      {
        throw CancellationError()
      }
      if error.isSessionTaskError || response.response == nil {
        throw HTTPError.network(
          error.underlyingError?.localizedDescription ?? error.localizedDescription)
      }
    }
    guard let http = response.response else {
      throw HTTPError.malformedResponse("\(path): no response")
    }
    return http
  }

  private struct MeRequest: Encodable {
    let query: String
  }

  private struct MeResponse: Decodable {
    let data: MeData?
    let errors: [MeError]?
  }

  private struct MeData: Decodable {
    let me: MeUser?
  }

  private struct MeUser: Decodable {
    let id: String
    let sites: [MeSite]
  }

  private struct MeSite: Decodable {
    let id: String
  }

  private struct MeError: Decodable {
    let message: String?
  }

  private struct TokenResponse: Decodable {
    let accessToken: String

    enum CodingKeys: String, CodingKey {
      case accessToken = "access_token"
    }
  }

  private struct TokenError: Decodable {
    let error: String
  }
}
