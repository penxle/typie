import Foundation
import Testing

@testable import Core

@Suite struct AppConfigTests {
  private let valid: [String: Any] = [
    "API_URL": "https://api.example.test",
    "AUTH_URL": "https://auth.example.test",
    "OIDC_CLIENT_ID": "client",
    "OIDC_CLIENT_SECRET": "secret",
    "KAKAO_NATIVE_APP_KEY": "kakao-key",
    "NAVER_CLIENT_ID": "naver-client",
    "NAVER_CLIENT_SECRET": "naver-secret",
  ]

  private func make(_ dictionary: [String: Any]) -> AppConfig {
    AppConfig(infoDictionary: dictionary)
  }

  @Test func loadsAllValues() {
    let config = make(valid)
    #expect(config.apiURL == URL(string: "https://api.example.test"))
    #expect(config.authURL == URL(string: "https://auth.example.test"))
    #expect(config.oidcClientID == "client")
    #expect(config.oidcClientSecret == "secret")
    #expect(config.kakaoAppKey == "kakao-key")
    #expect(config.naverClientID == "naver-client")
    #expect(config.naverClientSecret == "naver-secret")
  }
}
