import UIKit
import XCTest

@testable import Bridge

@MainActor final class EditorTextInputBridgeTests: XCTestCase {
  func testTraitsAreInstalledOnTheSuppliedViewBeforeItBecomesFirstResponder() {
    let input = LifecycleInputView()
    XCTAssertFalse(input.isFirstResponder)
    let generation = EditorTextInputBridge.install(on: input)
    defer { EditorTextInputBridge.uninstall(generation: generation) }

    XCTAssertEqual(input.smartQuotesType, .no)
    XCTAssertEqual(input.smartDashesType, .no)
    XCTAssertEqual(input.smartInsertDeleteType, .no)
  }

  func testOldConnectionCleanupDoesNotDeactivateNewConnection() {
    let oldInput = LifecycleInputView()
    let newInput = LifecycleInputView()
    let oldGeneration = EditorTextInputBridge.install(on: oldInput)
    let newGeneration = EditorTextInputBridge.install(on: newInput)
    EditorTextInputBridge.uninstall(generation: oldGeneration)

    XCTAssertEqual(oldInput.smartQuotesType, .default)
    XCTAssertEqual(newInput.smartQuotesType, .no)
    EditorTextInputBridge.uninstall(generation: newGeneration)
    XCTAssertEqual(newInput.smartQuotesType, .default)
  }

  func testFloatingCursorCallbacksFollowTheSuppliedConnectionAndItsCleanup() {
    let oldInput = LifecycleInputView()
    let newInput = LifecycleInputView()
    var events: [String] = []
    var displacement: CGPoint?
    let oldGeneration = EditorFloatingCursorBridge.install(
      on: oldInput, onBegin: { events.append("old") }, onUpdate: { _, _ in }, onEnd: {})
    let newGeneration = EditorFloatingCursorBridge.install(
      on: newInput, onBegin: { events.append("begin") },
      onUpdate: { x, y in displacement = CGPoint(x: x, y: y) },
      onEnd: { events.append("end") })
    EditorFloatingCursorBridge.clearHandlersForInstall(generation: oldGeneration)

    (oldInput as UITextInput).beginFloatingCursor?(at: .zero)
    XCTAssertEqual(oldInput.originalBegins, 1)
    (newInput as UITextInput).beginFloatingCursor?(at: CGPoint(x: 10, y: 20))
    (newInput as UITextInput).updateFloatingCursor?(at: CGPoint(x: 14, y: 17))
    (newInput as UITextInput).endFloatingCursor?()
    XCTAssertEqual(events, ["begin", "end"])
    XCTAssertEqual(displacement, CGPoint(x: 4, y: -3))
    XCTAssertEqual(newInput.originalBegins, 0)

    EditorFloatingCursorBridge.clearHandlersForInstall(generation: newGeneration)
    (newInput as UITextInput).beginFloatingCursor?(at: .zero)
    XCTAssertEqual(newInput.originalBegins, 1)
    XCTAssertEqual(events, ["begin", "end"])
  }
}

@MainActor private final class LifecycleInputView: UITextView {
  var originalBegins = 0

  override dynamic func beginFloatingCursor(at point: CGPoint) { originalBegins += 1 }
  override dynamic func updateFloatingCursor(at point: CGPoint) {}
  override dynamic func endFloatingCursor() {}
}
