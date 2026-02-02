import Flutter
import UIKit

class EditorInputView: NSObject, FlutterPlatformView {
  private let inputView: EditorTextInputView
  private let channel: FlutterMethodChannel

  init(frame: CGRect, messenger: FlutterBinaryMessenger, id: Int64) {
    inputView = EditorTextInputView(frame: frame)
    channel = FlutterMethodChannel(name: "co.typie.editor_input.\(id)", binaryMessenger: messenger)
    super.init()

    inputView.onInsertText = { [weak self] text in
      self?.channel.invokeMethod("insertText", arguments: ["text": text])
    }

    inputView.onDeleteBackward = { [weak self] in
      self?.channel.invokeMethod("deleteBackward", arguments: [String: Any]())
    }

    inputView.onSetMarkedText = { [weak self] text in
      self?.channel.invokeMethod("setMarkedText", arguments: ["text": text])
    }

    inputView.onUnmarkText = { [weak self] in
      self?.channel.invokeMethod("unmarkText", arguments: [String: Any]())
    }

    inputView.onCancelMarkedText = { [weak self] in
      self?.channel.invokeMethod("cancelMarkedText", arguments: [String: Any]())
    }

    inputView.onPerformAction = { [weak self] action in
      self?.channel.invokeMethod("performAction", arguments: ["action": action])
    }

    inputView.onShortcut = { [weak self] action in
      self?.channel.invokeMethod("shortcut", arguments: ["action": action])
    }

    channel.setMethodCallHandler { [weak self] call, result in
      guard let self = self else {
        result(FlutterMethodNotImplemented)
        return
      }
      switch call.method {
      case "activate":
        self.inputView.activate()
        result(nil)
      case "deactivate":
        self.inputView.deactivate()
        result(nil)
      case "releaseFocus":
        result(nil)
      case "resetInputContext":
        self.inputView.resetInputContext()
        result(nil)
      case "updateCursor":
        if let args = call.arguments as? [String: Any],
           let x = args["x"] as? Double,
           let y = args["y"] as? Double,
           let height = args["height"] as? Double {
          self.inputView.updateCursor(x: x, y: y, height: height)
        }
        result(nil)
      default:
        result(FlutterMethodNotImplemented)
      }
    }
  }

  func view() -> UIView {
    return inputView
  }
}

class EditorTextInputView: UIView, UITextInput {
  var onInsertText: ((String) -> Void)?
  var onDeleteBackward: (() -> Void)?
  var onSetMarkedText: ((String) -> Void)?
  var onUnmarkText: (() -> Void)?
  var onCancelMarkedText: (() -> Void)?
  var onPerformAction: ((String) -> Void)?
  var onShortcut: ((String) -> Void)?

  private var _markedText: String?

  private var cursorX: Double = 0
  private var cursorY: Double = 0
  private var cursorHeight: Double = 20

  override init(frame: CGRect) {
    super.init(frame: frame)
    backgroundColor = .clear
  }

  override func hitTest(_ point: CGPoint, with event: UIEvent?) -> UIView? {
    return nil
  }

  required init?(coder: NSCoder) {
    fatalError("init(coder:) has not been implemented")
  }

  func activate() {
    DispatchQueue.main.async { [weak self] in
      self?.becomeFirstResponder()
    }
  }

  func deactivate() {
    DispatchQueue.main.async { [weak self] in
      self?.resignFirstResponder()
    }
  }

  func updateCursor(x: Double, y: Double, height: Double) {
    cursorX = x
    cursorY = y
    cursorHeight = height
  }

  func resetInputContext() {
    _markedText = nil
    inputDelegate?.selectionWillChange(self)
    inputDelegate?.selectionDidChange(self)
  }

  override var canBecomeFirstResponder: Bool { true }

  // MARK: - UITextInputTraits

  var autocapitalizationType: UITextAutocapitalizationType = .none
  var autocorrectionType: UITextAutocorrectionType = .no
  var spellCheckingType: UITextSpellCheckingType = .no
  var smartQuotesType: UITextSmartQuotesType = .no
  var smartDashesType: UITextSmartDashesType = .no
  var smartInsertDeleteType: UITextSmartInsertDeleteType = .no

  // MARK: - Key Commands (shortcuts only)

  override var keyCommands: [UIKeyCommand]? { Self.cachedKeyCommands }

  private static let shortcutDefs: [(input: String, mods: UIKeyModifierFlags, action: String)] = [
    ("a", .command, "selectAll"),
    ("b", .command, "toggleBold"),
    ("i", .command, "toggleItalic"),
    ("u", .command, "toggleUnderline"),
    ("s", [.command, .shift], "toggleStrikethrough"),
    ("z", .command, "undo"),
    ("z", [.command, .shift], "redo"),
    ("\\", .command, "clearFormatting"),
    ("\t", [], "indent"),
    ("\t", .shift, "outdent"),
    ("\r", .command, "insertPageBreak"),
    ("\r", .shift, "insertHardBreak"),
    ("\u{8}", .command, "deleteToLineStart"),
    ("\u{8}", .alternate, "deleteWordBackward"),
  ]

  private static let cachedKeyCommands: [UIKeyCommand] = shortcutDefs.map { def in
    let cmd = UIKeyCommand(input: def.input, modifierFlags: def.mods, action: #selector(handleShortcut(_:)))
    cmd.wantsPriorityOverSystemBehavior = true
    return cmd
  }

  @objc private func handleShortcut(_ cmd: UIKeyCommand) {
    guard let input = cmd.input else { return }
    let mods = cmd.modifierFlags
    for def in Self.shortcutDefs {
      if def.input == input && def.mods == mods {
        if _markedText != nil {
          _markedText = nil
          onUnmarkText?()
        }
        onShortcut?(def.action)
        return
      }
    }
  }

  // MARK: - UIKeyInput

  var hasText: Bool { true }

  func insertText(_ text: String) {
    print("[EditorInput] insertText: '\(text)', markedText: '\(_markedText ?? "nil")'")
    if _markedText != nil {
      _markedText = nil
      onUnmarkText?()
    }

    if text == "\n" {
      onPerformAction?("newline")
      return
    }

    onInsertText?(text)
  }

  func deleteBackward() {
    print("[EditorInput] deleteBackward, markedText: '\(_markedText ?? "nil")'")
    if _markedText != nil {
      _markedText = nil
      onCancelMarkedText?()
      return
    }
    onDeleteBackward?()
  }

  // MARK: - UITextInput (Marked Text)

  var markedTextRange: UITextRange? {
    guard let markedText = _markedText else { return nil }
    return EditorTextRange(start: 0, end: markedText.count)
  }

  var markedTextStyle: [NSAttributedString.Key: Any]?

  func setMarkedText(_ markedText: String?, selectedRange: NSRange) {
    print("[EditorInput] setMarkedText: '\(markedText ?? "nil")', prev: '\(_markedText ?? "nil")'")

    if let text = markedText, !text.isEmpty {
      _markedText = text
      onSetMarkedText?(text)
    } else {
      if _markedText != nil {
        _markedText = nil
        onUnmarkText?()
      }
    }
  }

  func unmarkText() {
    print("[EditorInput] unmarkText, markedText: '\(_markedText ?? "nil")'")
    if _markedText != nil {
      _markedText = nil
      onUnmarkText?()
    }
  }

  // MARK: - UITextInput (Selection)

  var selectedTextRange: UITextRange? {
    get {
      let pos = _markedText?.count ?? 0
      return EditorTextRange(start: pos, end: pos)
    }
    set {}
  }

  // MARK: - UITextInput (Text Geometry)

  func firstRect(for range: UITextRange) -> CGRect {
    return CGRect(x: cursorX, y: cursorY, width: 1, height: cursorHeight)
  }

  func caretRect(for position: UITextPosition) -> CGRect {
    return CGRect(x: cursorX, y: cursorY, width: 1, height: cursorHeight)
  }

  // MARK: - UITextInput (Document)

  var beginningOfDocument: UITextPosition { EditorTextPosition(offset: 0) }
  var endOfDocument: UITextPosition { EditorTextPosition(offset: _markedText?.count ?? 0) }
  var inputDelegate: (any UITextInputDelegate)?
  var tokenizer: any UITextInputTokenizer { UITextInputStringTokenizer(textInput: self) }

  func text(in range: UITextRange) -> String? {
    return _markedText ?? ""
  }

  func replace(_ range: UITextRange, withText text: String) {}

  func textRange(from fromPosition: UITextPosition, to toPosition: UITextPosition) -> UITextRange? {
    guard let from = fromPosition as? EditorTextPosition,
          let to = toPosition as? EditorTextPosition else { return nil }
    return EditorTextRange(start: from.offset, end: to.offset)
  }

  func position(from position: UITextPosition, offset: Int) -> UITextPosition? {
    guard let pos = position as? EditorTextPosition else { return nil }
    let newOffset = pos.offset + offset
    let maxLen = _markedText?.count ?? 0
    if newOffset < 0 || newOffset > maxLen { return nil }
    return EditorTextPosition(offset: newOffset)
  }

  func position(from position: UITextPosition, in direction: UITextLayoutDirection, offset: Int) -> UITextPosition? {
    guard let pos = position as? EditorTextPosition else { return nil }
    let delta = (direction == .left || direction == .up) ? -offset : offset
    let newOffset = pos.offset + delta
    let maxLen = _markedText?.count ?? 0
    if newOffset < 0 || newOffset > maxLen { return nil }
    return EditorTextPosition(offset: newOffset)
  }

  func compare(_ position: UITextPosition, to other: UITextPosition) -> ComparisonResult {
    guard let pos1 = position as? EditorTextPosition,
          let pos2 = other as? EditorTextPosition else { return .orderedSame }
    if pos1.offset < pos2.offset { return .orderedAscending }
    if pos1.offset > pos2.offset { return .orderedDescending }
    return .orderedSame
  }

  func offset(from: UITextPosition, to toPosition: UITextPosition) -> Int {
    guard let fromPos = from as? EditorTextPosition,
          let toPos = toPosition as? EditorTextPosition else { return 0 }
    return toPos.offset - fromPos.offset
  }

  func selectionRects(for range: UITextRange) -> [UITextSelectionRect] { [] }

  func closestPosition(to point: CGPoint) -> UITextPosition? {
    return EditorTextPosition(offset: _markedText?.count ?? 0)
  }

  func closestPosition(to point: CGPoint, within range: UITextRange) -> UITextPosition? {
    return EditorTextPosition(offset: _markedText?.count ?? 0)
  }

  func characterRange(at point: CGPoint) -> UITextRange? { nil }

  func baseWritingDirection(for position: UITextPosition, in direction: UITextStorageDirection) -> NSWritingDirection {
    return .leftToRight
  }

  func setBaseWritingDirection(_ writingDirection: NSWritingDirection, for range: UITextRange) {}

  func position(within range: UITextRange, farthestIn direction: UITextLayoutDirection) -> UITextPosition? {
    guard let editorRange = range as? EditorTextRange else { return nil }
    if direction == .left || direction == .up {
      return EditorTextPosition(offset: editorRange.startOffset)
    } else {
      return EditorTextPosition(offset: editorRange.endOffset)
    }
  }

  func characterRange(byExtending position: UITextPosition, in direction: UITextLayoutDirection) -> UITextRange? {
    guard let pos = position as? EditorTextPosition else { return nil }
    let maxLen = _markedText?.count ?? 0
    if direction == .left || direction == .up {
      return EditorTextRange(start: 0, end: pos.offset)
    } else {
      return EditorTextRange(start: pos.offset, end: maxLen)
    }
  }

  var textInputView: UIView { self }
}

class EditorTextPosition: UITextPosition {
  let offset: Int
  init(offset: Int) {
    self.offset = offset
  }
}

class EditorTextRange: UITextRange {
  private let _start: EditorTextPosition
  private let _end: EditorTextPosition

  var startOffset: Int { _start.offset }
  var endOffset: Int { _end.offset }

  init(start: Int, end: Int) {
    _start = EditorTextPosition(offset: start)
    _end = EditorTextPosition(offset: end)
  }

  override var start: UITextPosition { _start }
  override var end: UITextPosition { _end }
  override var isEmpty: Bool { _start.offset == _end.offset }
}
