import UIKit
import XCTest

@testable import Bridge

@MainActor final class EditorDocumentNavigationTests: XCTestCase {
  func testPendingTextIsFlushedBeforeNativeMovement() {
    let navigation = EditorDocumentNavigation()
    let input = makeInput()
    var pendingText = true
    _ = navigation.perform(
      on: input, backward: false, extending: false, isCurrent: { true },
      move: {
        XCTAssertFalse(pendingText, "A prior text edit must not be treated as this arrow action")
        input.selectedRange = NSRange(location: 1, length: 0)
        return nil
      },
      flush: { pendingText = false },
      endComposition: {}
    )
  }

  func testMovementIsAppliedAndCompositionEndedBeforeTheNextInput() {
    let navigation = EditorDocumentNavigation()
    let input = makeInput()
    let history = NSObject()
    var events: [String] = []
    let result = navigation.perform(
      on: input, backward: false, extending: false, isCurrent: { true },
      move: {
        events.append("move")
        input.selectedRange = NSRange(location: 1, length: 0)
        return history
      },
      flush: { events.append("flush") },
      endComposition: {
        XCTAssertEqual(input.selectedRange, NSRange(location: 1, length: 0))
        events.append("end")
      }
    )
    events.append("next input")

    XCTAssertTrue(result === history)
    XCTAssertEqual(events, ["flush", "move", "flush", "end", "flush", "next input"])
  }

  func testOnlyTheActionsSelectionFlushCanConsumeOneDocumentMovement() {
    for backward in [true, false] {
      for extending in [true, false] {
        let navigation = EditorDocumentNavigation()
        let input = makeInput()
        let history = NSObject()
        var moved = false
        var routed = 0
        let result = navigation.perform(
          on: input, backward: backward, extending: extending, isCurrent: { true },
          move: {
            navigation.takeMovement { _, _ in XCTFail("Prior edits are not navigation") }
            moved = true
            input.selectedRange = NSRange(location: 1, length: extending ? 1 : 0)
            return history
          },
          flush: {
            navigation.takeMovement { actualBackward, actualExtending in
              XCTAssertTrue(moved)
              XCTAssertEqual(actualBackward, backward)
              XCTAssertEqual(actualExtending, extending)
              routed += 1
              navigation.takeMovement { _, _ in XCTFail("Reentrant edits must not move twice") }
            }
          },
          endComposition: {
            navigation.takeMovement { _, _ in XCTFail("Composition commit is not navigation") }
          }
        )
        XCTAssertEqual(routed, 1)
        XCTAssertTrue(result === history)
        navigation.takeMovement { _, _ in XCTFail("Movement cannot escape its native action") }
      }
    }
  }

  func testMarkedTextAndCandidateMovementCannotRouteDocumentNavigation() {
    for startsMarked in [true, false] {
      let navigation = EditorDocumentNavigation()
      let input = makeInput()
      input.hasMarkedText = startsMarked
      _ = navigation.perform(
        on: input, backward: true, extending: false, isCurrent: { true },
        move: {
          input.hasMarkedText = true
          input.selectedRange = NSRange(location: 1, length: 0)
          return nil
        },
        flush: {
          navigation.takeMovement { _, _ in XCTFail("Marked text belongs to the input method") }
        },
        endComposition: { XCTFail("Marked text must not be committed") }
      )
      navigation.takeMovement { _, _ in XCTFail("Candidate action cannot leak navigation") }
    }
  }

  func testHostKeepingTheCaretAtDocumentEndDoesNotEndComposition() {
    let navigation = EditorDocumentNavigation()
    let input = makeInput()
    _ = navigation.perform(
      on: input, backward: false, extending: false, isCurrent: { true },
      move: {
        input.selectedRange = NSRange(location: 3, length: 0)
        return nil
      },
      flush: {
        navigation.takeMovement { _, _ in input.selectedRange = NSRange(location: 2, length: 0) }
      },
      endComposition: { XCTFail("A rejected move did not leave the caret") }
    )
    XCTAssertEqual(input.selectedRange, NSRange(location: 2, length: 0))
  }

  func testShiftRangesMarkedTextAndUnmovedCaretsDoNotEndComposition() {
    for scenario in 0..<6 {
      let navigation = EditorDocumentNavigation()
      let input = makeInput()
      if scenario == 1 { input.hasMarkedText = true }
      if scenario == 2 { input.selectedRange = NSRange(location: 1, length: 1) }
      var events: [String] = []

      _ = navigation.perform(
        on: input, backward: false, extending: scenario == 0, isCurrent: { true },
        move: {
          events.append("move")
          if scenario != 3 { input.selectedRange = NSRange(location: 1, length: 0) }
          if scenario == 4 { input.hasMarkedText = true }
          if scenario == 5 { input.selectedRange = NSRange(location: 1, length: 1) }
          return nil
        },
        flush: { events.append("flush") }, endComposition: { events.append("end") }
      )

      let expected =
        scenario == 1 ? ["move"] : scenario == 4 ? ["flush", "move"] : ["flush", "move", "flush"]
      XCTAssertEqual(events, expected, "scenario \(scenario)")
    }
  }

  func testExtendingMovementReconcilesSelectionBeforeReturningHistory() {
    let navigation = EditorDocumentNavigation()
    let input = makeInput()
    input.text = "ab\n\ncd"
    input.selectedRange = NSRange(location: 6, length: 0)
    let history = NSObject()
    var events: [String] = []

    let result = navigation.perform(
      on: input, backward: false, extending: true, isCurrent: { true },
      move: {
        events.append("move")
        input.selectedRange = NSRange(location: 3, length: 3)
        return history
      },
      flush: {
        events.append("flush")
        // Host normalization skips the non-insertable paragraph separator.
        input.selectedRange = NSRange(location: 2, length: 4)
      },
      endComposition: { XCTFail("Extending selection must not end composition") }
    )
    events.append("return")

    XCTAssertEqual(input.selectedRange, NSRange(location: 2, length: 4))
    XCTAssertEqual(events, ["flush", "move", "flush", "return"])
    XCTAssertTrue(result === history)
  }

  func testSessionReplacementDuringMovementOrFlushPreventsCompositionReset() {
    for replaceDuringFlush in [false, true] {
      let navigation = EditorDocumentNavigation()
      let input = makeInput()
      var current = true
      var moved = false
      var events: [String] = []

      _ = navigation.perform(
        on: input, backward: false, extending: false, isCurrent: { current },
        move: {
          moved = true
          input.selectedRange = NSRange(location: 1, length: 0)
          if !replaceDuringFlush { current = false }
          return nil
        },
        flush: {
          events.append("flush")
          if moved && replaceDuringFlush { current = false }
        },
        endComposition: { events.append("end") }
      )

      XCTAssertEqual(events, replaceDuringFlush ? ["flush", "flush"] : ["flush"])
    }
  }

  func testSessionReplacementWhileDrainingPriorTextDoesNotStartTheOldMovement() {
    let navigation = EditorDocumentNavigation()
    let input = makeInput()
    var current = true
    _ = navigation.perform(
      on: input, backward: true, extending: false, isCurrent: { current },
      move: {
        XCTFail("The prior edit replaced the input session")
        return nil
      },
      flush: { current = false },
      endComposition: { XCTFail("A replaced session cannot commit composition") }
    )
    navigation.takeMovement { _, _ in XCTFail("The replacement session cannot inherit movement") }
  }

  func testReentrantNativeMovementRunsOnceWithoutAnotherCompositionReset() {
    let navigation = EditorDocumentNavigation()
    let input = makeInput()
    var events: [String] = []
    _ = navigation.perform(
      on: input, backward: false, extending: false, isCurrent: { true },
      move: {
        input.selectedRange = NSRange(location: 1, length: 0)
        return nil
      },
      flush: { events.append("flush") },
      endComposition: {
        events.append("end")
        _ = navigation.perform(
          on: input, backward: false, extending: false, isCurrent: { true },
          move: {
            events.append("nested move")
            return nil
          },
          flush: { XCTFail("nested flush") }, endComposition: { XCTFail("nested reset") }
        )
      }
    )

    XCTAssertEqual(events, ["flush", "flush", "end", "nested move", "flush"])
  }

  func testHostReconciliationToMarkedTextOrARangePreventsCompositionReset() {
    for marked in [false, true] {
      let navigation = EditorDocumentNavigation()
      let input = makeInput()
      var events: [String] = []
      var moved = false
      _ = navigation.perform(
        on: input, backward: false, extending: false, isCurrent: { true },
        move: {
          moved = true
          input.selectedRange = NSRange(location: 1, length: 0)
          return nil
        },
        flush: {
          events.append("flush")
          guard moved else { return }
          if marked {
            input.hasMarkedText = true
          } else {
            input.selectedRange = NSRange(location: 0, length: 1)
          }
        },
        endComposition: { events.append("end") }
      )
      XCTAssertEqual(events, ["flush", "flush"])
    }
  }

  func testEndingCompositionCannotFlushAReplacementSession() {
    let navigation = EditorDocumentNavigation()
    let input = makeInput()
    var current = true
    var events: [String] = []
    _ = navigation.perform(
      on: input, backward: false, extending: false, isCurrent: { current },
      move: {
        input.selectedRange = NSRange(location: 1, length: 0)
        return nil
      },
      flush: { events.append("flush") },
      endComposition: {
        events.append("end")
        current = false
      }
    )
    XCTAssertEqual(events, ["flush", "flush", "end"])
  }

  private func makeInput() -> NavigationTextView {
    let input = NavigationTextView()
    input.text = "abc"
    input.selectedRange = NSRange(location: 2, length: 0)
    return input
  }
}

@MainActor private final class NavigationTextView: UITextView {
  var hasMarkedText = false

  override var markedTextRange: UITextRange? {
    hasMarkedText ? textRange(from: beginningOfDocument, to: endOfDocument) : nil
  }
}
