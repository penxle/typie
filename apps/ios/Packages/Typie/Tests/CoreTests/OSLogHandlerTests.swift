import Logging
import Testing
import os

@testable import Core

struct OSLogHandlerTests {
  @Test func splitsTheLabelAtTheLastDot() {
    let destination = OSLogHandler.destination(for: "co.typie.subscription")
    #expect(destination.subsystem == "co.typie")
    #expect(destination.category == "subscription")
  }

  @Test func labelsWithoutADotUseTheDefaultCategory() {
    let destination = OSLogHandler.destination(for: "typie")
    #expect(destination.subsystem == "typie")
    #expect(destination.category == "default")
  }

  @Test func mapsLevelsOntoOSLogTypes() {
    #expect(OSLogHandler.type(for: .trace) == .debug)
    #expect(OSLogHandler.type(for: .debug) == .debug)
    #expect(OSLogHandler.type(for: .info) == .info)
    #expect(OSLogHandler.type(for: .notice) == .default)
    #expect(OSLogHandler.type(for: .warning) == .default)
    #expect(OSLogHandler.type(for: .error) == .error)
    #expect(OSLogHandler.type(for: .critical) == .fault)
  }

  @Test func rendersMetadataSortedByKey() {
    #expect(OSLogHandler.render(["b": "2", "a": "1"]) == "a=1 b=2")
  }

  @Test func eventDetailsLayerHandlerProviderAndEventMetadataThenTheError() {
    struct Failure: Error {}
    var handler = OSLogHandler(label: "co.typie.test")
    handler.metadata = ["a": "handler", "b": "handler"]
    handler.metadataProvider = Logging.Logger.MetadataProvider {
      ["b": "provider", "c": "provider"]
    }
    let event = LogEvent(
      level: .notice, message: "m", error: Failure(), metadata: ["c": "event"], source: nil,
      file: #fileID, function: #function, line: #line)

    #expect(handler.details(of: event) == "a=handler b=provider c=event error=Failure()")
  }

  @Test func eventsWithoutMetadataHaveNoDetails() {
    let handler = OSLogHandler(label: "co.typie.test")
    let event = LogEvent(
      level: .notice, message: "m", metadata: nil, source: nil, file: #fileID,
      function: #function, line: #line)

    #expect(handler.details(of: event) == nil)
  }
}
