import Foundation
import Testing

@testable import Core

@Suite struct DateTimeParsingTests {
  @Test func parsesServerDateTime() throws {
    let parsed = try #require(parseDateTime("2027-01-02T03:04:05.678Z"))
    #expect(abs(parsed.timeIntervalSince1970 - 1_798_859_045.678) < 0.001)
    #expect(parseDateTime("2027-01-02T03:04:05Z") != nil)
    #expect(parseDateTime("not-a-date") == nil)
  }
}
