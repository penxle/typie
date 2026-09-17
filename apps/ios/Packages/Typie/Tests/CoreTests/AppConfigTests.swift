import Foundation
import Testing

@testable import Core

@Suite struct AppConfigTests {
  private let valid: [String: Any] = [
    "API_URL": "https://api.example.test",
    "AUTH_URL": "https://auth.example.test",
    "OIDC_CLIENT_ID": "client",
    "KAKAO_NATIVE_APP_KEY": "kakao-key",
    "NAVER_CLIENT_ID": "naver-client",
  ]

  private func make(_ dictionary: [String: Any]) throws -> AppConfig {
    try AppConfig(
      infoDictionary: dictionary, oidcClientSecret: "secret", naverClientSecret: "naver-secret")
  }

  @Test func loadsAllValues() throws {
    let config = try make(valid)
    #expect(config.apiURL == URL(string: "https://api.example.test"))
    #expect(config.authURL == URL(string: "https://auth.example.test"))
    #expect(config.oidcClientID == "client")
    #expect(config.oidcClientSecret == "secret")
    #expect(config.kakaoAppKey == "kakao-key")
    #expect(config.naverClientID == "naver-client")
    #expect(config.naverClientSecret == "naver-secret")
  }

  @Test func missingKeyThrows() {
    var dictionary = valid
    dictionary["AUTH_URL"] = nil
    #expect(throws: AppConfigError.missing("AUTH_URL")) { try make(dictionary) }
  }

  @Test func missingKakaoKeyThrows() {
    var dictionary = valid
    dictionary["KAKAO_NATIVE_APP_KEY"] = nil
    #expect(throws: AppConfigError.missing("KAKAO_NATIVE_APP_KEY")) { try make(dictionary) }
  }

  @Test func blankValueCountsAsMissing() {
    var dictionary = valid
    dictionary["OIDC_CLIENT_ID"] = "  "
    #expect(throws: AppConfigError.missing("OIDC_CLIENT_ID")) { try make(dictionary) }
  }

  @Test func blankSecretCountsAsMissing() {
    #expect(throws: AppConfigError.missing("OIDC_CLIENT_SECRET")) {
      try AppConfig(
        infoDictionary: valid, oidcClientSecret: "  ", naverClientSecret: "naver-secret")
    }
  }

  @Test func blankNaverSecretCountsAsMissing() {
    #expect(throws: AppConfigError.missing("NAVER_CLIENT_SECRET")) {
      try AppConfig(infoDictionary: valid, oidcClientSecret: "secret", naverClientSecret: " ")
    }
  }

  @Test func urlWithoutSchemeThrows() {
    var dictionary = valid
    dictionary["API_URL"] = "api.example.test"
    #expect(throws: AppConfigError.invalidURL("API_URL")) { try make(dictionary) }
  }

  @Test func trimsSurroundingNewlines() throws {
    var dictionary = valid
    dictionary["OIDC_CLIENT_ID"] = "client\n"
    #expect(try make(dictionary).oidcClientID == "client")
  }

  @Test func nonHTTPSchemeThrows() {
    var dictionary = valid
    dictionary["AUTH_URL"] = "file://auth.example.test"
    #expect(throws: AppConfigError.invalidURL("AUTH_URL")) { try make(dictionary) }
  }
}
