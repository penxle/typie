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
  var onPerformAction: ((String) -> Void)?
  var onShortcut: ((String) -> Void)?

  private var _markedText: String?
  private var _markedTextRange: UITextRange?
  private var _selectionOffset: Int = 0
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
    _markedTextRange = nil
    _selectionOffset += 1
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
          _markedTextRange = nil
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
    if text == "\n" {
      onPerformAction?("newline")
      return
    }

    if _markedText != nil {
      _markedText = nil
      _markedTextRange = nil
      onUnmarkText?()
      return
    }
    onInsertText?(text)
  }

  func deleteBackward() {
    if _markedText != nil {
      _markedText = nil
      _markedTextRange = nil
      onUnmarkText?()
      return
    }
    onDeleteBackward?()
  }

  // MARK: - UITextInput (Marked Text)

  var markedTextRange: UITextRange? { _markedTextRange }
  var markedTextStyle: [NSAttributedString.Key: Any]?

  func setMarkedText(_ markedText: String?, selectedRange: NSRange) {
    if let text = markedText, !text.isEmpty {
      _markedText = text
      _markedTextRange = EditorTextRange(start: 0, end: text.count)
      onSetMarkedText?(text)
    } else {
      let hadMarked = _markedText != nil
      _markedText = nil
      _markedTextRange = nil
      if hadMarked {
        onUnmarkText?()
      }
    }
  }

  func unmarkText() {
    _markedText = nil
    _markedTextRange = nil
    onUnmarkText?()
  }

  // MARK: - UITextInput (Selection)

  var selectedTextRange: UITextRange? {
    get { EditorTextRange(start: _selectionOffset, end: _selectionOffset) }
    set {}
  }

  // MARK: - UITextInput (Text Geometry)

  func firstRect(for range: UITextRange) -> CGRect {
    return CGRect(x: cursorX, y: cursorY, width: 1, height: cursorHeight)
  }

  func caretRect(for position: UITextPosition) -> CGRect {
    return CGRect(x: cursorX, y: cursorY, width: 1, height: cursorHeight)
  }

  // MARK: - UITextInput (Required stubs)

  var beginningOfDocument: UITextPosition { EditorTextPosition(offset: 0) }
  var endOfDocument: UITextPosition { EditorTextPosition(offset: 0) }
  var inputDelegate: (any UITextInputDelegate)?
  var tokenizer: any UITextInputTokenizer { UITextInputStringTokenizer(textInput: self) }

  func text(in range: UITextRange) -> String? { nil }
  func replace(_ range: UITextRange, withText text: String) {}

  func textRange(from fromPosition: UITextPosition, to toPosition: UITextPosition) -> UITextRange? {
    return EditorTextRange(start: 0, end: 0)
  }

  func position(from position: UITextPosition, offset: Int) -> UITextPosition? {
    return EditorTextPosition(offset: 0)
  }

  func position(from position: UITextPosition, in direction: UITextLayoutDirection, offset: Int) -> UITextPosition? {
    return EditorTextPosition(offset: 0)
  }

  func compare(_ position: UITextPosition, to other: UITextPosition) -> ComparisonResult {
    return .orderedSame
  }

  func offset(from: UITextPosition, to toPosition: UITextPosition) -> Int { 0 }

  func selectionRects(for range: UITextRange) -> [UITextSelectionRect] { [] }
  func closestPosition(to point: CGPoint) -> UITextPosition? { EditorTextPosition(offset: 0) }
  func closestPosition(to point: CGPoint, within range: UITextRange) -> UITextPosition? { EditorTextPosition(offset: 0) }
  func characterRange(at point: CGPoint) -> UITextRange? { nil }

  func baseWritingDirection(for position: UITextPosition, in direction: UITextStorageDirection) -> NSWritingDirection {
    return .leftToRight
  }

  func setBaseWritingDirection(_ writingDirection: NSWritingDirection, for range: UITextRange) {}

  func position(within range: UITextRange, farthestIn direction: UITextLayoutDirection) -> UITextPosition? {
    return EditorTextPosition(offset: 0)
  }

  func characterRange(byExtending position: UITextPosition, in direction: UITextLayoutDirection) -> UITextRange? {
    return EditorTextRange(start: 0, end: 0)
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

  init(start: Int, end: Int) {
    _start = EditorTextPosition(offset: start)
    _end = EditorTextPosition(offset: end)
  }

  override var start: UITextPosition { _start }
  override var end: UITextPosition { _end }
  override var isEmpty: Bool { _start.offset == _end.offset }
}
