import Foundation

public enum AppConfigError: Error, Equatable {
  case missing(String)
  case invalidURL(String)
}

public struct AppConfig: Sendable, Equatable {
  public let apiURL: URL
  public let authURL: URL
  public let oidcClientID: String
  public let oidcClientSecret: String
  public let kakaoAppKey: String
  public let naverClientID: String
  public let naverClientSecret: String

  public init(infoDictionary: [String: Any], oidcClientSecret: String, naverClientSecret: String)
    throws
  {
    apiURL = try Self.url("API_URL", in: infoDictionary)
    authURL = try Self.url("AUTH_URL", in: infoDictionary)
    oidcClientID = try Self.string("OIDC_CLIENT_ID", in: infoDictionary)
    kakaoAppKey = try Self.string("KAKAO_NATIVE_APP_KEY", in: infoDictionary)
    naverClientID = try Self.string("NAVER_CLIENT_ID", in: infoDictionary)
    self.oidcClientSecret = try Self.secret(oidcClientSecret, named: "OIDC_CLIENT_SECRET")
    self.naverClientSecret = try Self.secret(naverClientSecret, named: "NAVER_CLIENT_SECRET")
  }

  public static func load(
    bundle: Bundle = .main, oidcClientSecret: String, naverClientSecret: String
  ) throws -> AppConfig {
    try AppConfig(
      infoDictionary: bundle.infoDictionary ?? [:], oidcClientSecret: oidcClientSecret,
      naverClientSecret: naverClientSecret)
  }

  private static func secret(_ value: String, named name: String) throws -> String {
    guard !value.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
      throw AppConfigError.missing(name)
    }
    return value
  }

  private static func string(_ key: String, in dictionary: [String: Any]) throws -> String {
    guard let value = (dictionary[key] as? String)?.trimmingCharacters(in: .whitespacesAndNewlines),
      !value.isEmpty
    else { throw AppConfigError.missing(key) }
    return value
  }

  private static func url(_ key: String, in dictionary: [String: Any]) throws -> URL {
    let value = try string(key, in: dictionary)
    guard let url = URL(string: value), let scheme = url.scheme?.lowercased(),
      scheme == "https" || scheme == "http", url.host() != nil
    else {
      throw AppConfigError.invalidURL(key)
    }
    return url
  }
}
