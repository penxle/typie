import UIKit

// Completes a native document-navigation action before another input can begin.
// Candidate and marked-text navigation remain entirely owned by the input method.
@MainActor final class EditorDocumentNavigation {
  private var isPerforming = false
  private var pendingMovement: (backward: Bool, extending: Bool)?

  // Available only while flushing the selection produced by one native action.
  // Consume before invoking the host: reentrant edits must not move a second time.
  func takeMovement(_ consume: (Bool, Bool) -> Void) {
    guard let movement = pendingMovement else { return }
    pendingMovement = nil
    consume(movement.backward, movement.extending)
  }

  func perform(
    on input: UITextInput,
    backward: Bool,
    extending: Bool,
    isCurrent: () -> Bool,
    move: () -> AnyObject?,
    flush: () -> Void,
    endComposition: () -> Void
  ) -> AnyObject? {
    guard isCurrent(), !isPerforming else { return move() }
    isPerforming = true
    defer {
      pendingMovement = nil
      isPerforming = false
    }

    guard input.markedTextRange == nil else { return move() }
    // Do not mix a previously pending text edit with this navigation action.
    flush()
    guard isCurrent() else { return nil }

    let before = collapsedCaret(input)
    let hadMarkedText = input.markedTextRange != nil
    let result = move()
    guard isCurrent(), !hadMarkedText, input.markedTextRange == nil
    else { return result }

    // Reconcile inside UIKit's native action, including Shift selection. A deferred
    // paragraph-boundary correction looks external and discards its arrow history.
    pendingMovement = (backward, extending)
    flush()
    pendingMovement = nil

    // Synchronizing a selection is not permission to end an input method's composition.
    guard isCurrent(), !extending, let before, let after = collapsedCaret(input), before != after,
      input.markedTextRange == nil
    else { return result }
    endComposition()
    if isCurrent() {
      flush()
    }
    return result
  }

  private func collapsedCaret(_ input: UITextInput) -> Int? {
    guard let range = input.selectedTextRange, range.isEmpty else { return nil }
    return input.offset(from: input.beginningOfDocument, to: range.start)
  }
}
