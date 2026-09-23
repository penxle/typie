import Foundation

public struct AppConfig: Sendable, Equatable {
  public let apiURL: URL
  public let authURL: URL
  public let wsURL: URL
  public let oidcClientID: String
  public let oidcClientSecret: String
  public let kakaoAppKey: String
  public let naverClientID: String
  public let naverClientSecret: String

  public init(infoDictionary: [String: Any]) {
    apiURL = Self.url("API_URL", in: infoDictionary)
    authURL = Self.url("AUTH_URL", in: infoDictionary)
    wsURL = Self.url("WS_URL", in: infoDictionary)
    oidcClientID = Self.string("OIDC_CLIENT_ID", in: infoDictionary)
    kakaoAppKey = Self.string("KAKAO_NATIVE_APP_KEY", in: infoDictionary)
    naverClientID = Self.string("NAVER_CLIENT_ID", in: infoDictionary)
    oidcClientSecret = Self.string("OIDC_CLIENT_SECRET", in: infoDictionary)
    naverClientSecret = Self.string("NAVER_CLIENT_SECRET", in: infoDictionary)
  }

  public static func load(bundle: Bundle = .main) -> AppConfig {
    AppConfig(infoDictionary: bundle.infoDictionary ?? [:])
  }

  private static func string(_ key: String, in dictionary: [String: Any]) -> String {
    dictionary[key] as! String
  }

  private static func url(_ key: String, in dictionary: [String: Any]) -> URL {
    URL(string: string(key, in: dictionary))!
  }
}
