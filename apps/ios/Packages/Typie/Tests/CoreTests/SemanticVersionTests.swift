import Testing

@testable import Core

@Suite struct SemanticVersionTests {
  @Test func olderMajorIsOlder() {
    #expect(SemanticVersion.isOlder(current: "2.1.6", required: "3.0.0"))
  }

  @Test func equalVersionsAreNotOlder() {
    #expect(!SemanticVersion.isOlder(current: "3.0.0", required: "3.0.0"))
  }

  @Test func missingComponentsCountAsZero() {
    #expect(!SemanticVersion.isOlder(current: "3.0.0", required: "3.0"))
    #expect(SemanticVersion.isOlder(current: "3", required: "3.0.1"))
  }

  @Test func laterMinorBeatsLaterPatch() {
    #expect(!SemanticVersion.isOlder(current: "3.1", required: "3.0.9"))
  }

  @Test func nonNumericComponentsCountAsZero() {
    #expect(SemanticVersion.isOlder(current: "a.b", required: "0.1"))
    #expect(!SemanticVersion.isOlder(current: "a.b", required: "0"))
  }
}
