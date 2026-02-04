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

    inputView.onFocusLost = { [weak self] in
      self?.channel.invokeMethod("focusLost", arguments: [String: Any]())
    }

    inputView.onReplaceBackward = { [weak self] length, text in
      self?.channel.invokeMethod("replaceBackward", arguments: ["length": length, "text": text])
    }

    inputView.onFloatingCursorBegin = { [weak self] in
      self?.channel.invokeMethod("floatingCursorBegin", arguments: [String: Any]())
    }

    inputView.onFloatingCursorUpdate = { [weak self] dx, dy in
      self?.channel.invokeMethod("floatingCursorUpdate", arguments: ["dx": dx, "dy": dy])
    }

    inputView.onFloatingCursorEnd = { [weak self] in
      self?.channel.invokeMethod("floatingCursorEnd", arguments: [String: Any]())
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
          let precedingCharWidths = args["precedingCharWidths"] as? [Double]
          self.inputView.updateCursor(x: x, y: y, height: height, precedingCharWidths: precedingCharWidths)
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
  var onFocusLost: (() -> Void)?
  var onReplaceBackward: ((Int, String) -> Void)?
  var onFloatingCursorBegin: (() -> Void)?
  var onFloatingCursorUpdate: ((Double, Double) -> Void)?
  var onFloatingCursorEnd: (() -> Void)?

  private var _markedText: String?
  private var _cursor: Int = 0
  private var _isDeactivating: Bool = false
  private var _shadowText: String = ""
  private var _precedingCharWidths: [Double] = []

  private var cursorX: Double = 0
  private var cursorY: Double = 0
  private var cursorHeight: Double = 20
  private var _floatingCursorStart: CGPoint?



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
      self?._isDeactivating = true
      self?.resignFirstResponder()
      self?._isDeactivating = false
    }
  }

  @discardableResult
  override func resignFirstResponder() -> Bool {
    let result = super.resignFirstResponder()
    if result && !_isDeactivating {
      onFocusLost?()
    }
    return result
  }

  func updateCursor(x: Double, y: Double, height: Double, precedingCharWidths: [Double]? = nil) {
    cursorX = x
    cursorY = y
    cursorHeight = height
    if let widths = precedingCharWidths {
      _precedingCharWidths = widths
    }
  }

  func resetInputContext() {
    if _markedText != nil {
      _markedText = nil
      onUnmarkText?()
    }
    _shadowText = ""
    inputDelegate?.textWillChange(self)
    inputDelegate?.textDidChange(self)
  }

  override var canBecomeFirstResponder: Bool { true }

  // MARK: - Floating Cursor (keyboard trackpad mode)

  func beginFloatingCursor(at point: CGPoint) {
    if _markedText != nil {
      _markedText = nil
      onUnmarkText?()
    }
    _floatingCursorStart = point
    onFloatingCursorBegin?()
  }

  func updateFloatingCursor(at point: CGPoint) {
    guard let start = _floatingCursorStart else { return }
    let dx = Double(point.x - start.x)
    let dy = Double(point.y - start.y)
    onFloatingCursorUpdate?(dx, dy)
  }

  func endFloatingCursor() {
    _floatingCursorStart = nil
    onFloatingCursorEnd?()
  }

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

        _shadowText = ""
        _cursor = 0
        inputDelegate?.textWillChange(self)
        inputDelegate?.selectionWillChange(self)
        inputDelegate?.textDidChange(self)
        inputDelegate?.selectionDidChange(self)
        
        return
      }
    }
  }

  // MARK: - UIKeyInput

  var hasText: Bool { true }

  private var isSoftKeyboardShiftActive: Bool {
    guard let cls = NSClassFromString("UIKeyboardImpl") as? NSObject.Type,
          let instance = cls.perform(NSSelectorFromString("activeInstance"))?.takeUnretainedValue() as? NSObject else {
      return false
    }
    return instance.perform(NSSelectorFromString("isShifted")) != nil
  }

  func insertText(_ text: String) {
    if _markedText != nil {
      _markedText = nil
    }

    if text == "\n" {
      if isSoftKeyboardShiftActive {
        onShortcut?("insertHardBreak")
      } else {
        onPerformAction?("newline")
      }
      _shadowText = ""
      _cursor = 0
      return
    }

    _shadowText.append(text)
    if _shadowText.count > 64 {
      _shadowText = String(_shadowText.suffix(64))
    }
    _cursor += text.count
    
    inputDelegate?.textWillChange(self)
    inputDelegate?.textDidChange(self)
    
    onInsertText?(text)
  }

  func deleteBackward() {
    if _markedText != nil {
      _markedText = nil
      onCancelMarkedText?()
      return
    }

    _shadowText = ""
    
    _cursor -= 1
    if _cursor < 0 {
      _cursor = 0
    }

    if !_shadowText.isEmpty {
      _shadowText.removeLast()
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
    if let text = markedText, !text.isEmpty {
      _markedText = text
      onSetMarkedText?(text)
    } else {
      if _markedText != nil {
        _markedText = nil
        onCancelMarkedText?()
      }
    }
  }

  func unmarkText() {
    if _markedText != nil {
      _markedText = nil
      onUnmarkText?()
    }
  }

  // MARK: - UITextInput (Selection)

  var selectedTextRange: UITextRange? {
    get {
      if let markedText = _markedText {
        let pos = markedText.count
        return EditorTextRange(start: pos, end: pos)
      }
      let pos = _shadowText.count
      return EditorTextRange(start: pos, end: pos)
    }
    set {}
  }

  // MARK: - UITextInput (Text Geometry)

  func firstRect(for range: UITextRange) -> CGRect {
    guard let editorRange = range as? EditorTextRange else {
      return CGRect(x: cursorX, y: cursorY, width: 1, height: cursorHeight)
    }
    
    let shadowLen = _shadowText.count
    let rangeStart = editorRange.startOffset
    let rangeEnd = editorRange.endOffset
    
    if shadowLen == 0 || rangeStart >= shadowLen {
      return CGRect(x: cursorX, y: cursorY, width: 1, height: cursorHeight)
    }
    
    let availableWidths = min(_precedingCharWidths.count, shadowLen)
    
    if availableWidths == 0 {
      return CGRect(x: cursorX, y: cursorY, width: 1, height: cursorHeight)
    }
    
    var startX = cursorX
    for i in stride(from: shadowLen - 1, through: rangeStart, by: -1) {
      if i < availableWidths {
        startX -= _precedingCharWidths[i]
      }
    }
    
    var width: Double = 0
    for i in rangeStart..<min(rangeEnd, availableWidths) {
      width += _precedingCharWidths[i]
    }
    
    if width < 1 {
      width = 1
    }
    
    return CGRect(x: startX, y: cursorY, width: width, height: cursorHeight)
  }


  func caretRect(for position: UITextPosition) -> CGRect {
    return CGRect(x: cursorX, y: cursorY, width: 1, height: cursorHeight)
  }

  // MARK: - UITextInput (Document)

  var beginningOfDocument: UITextPosition { EditorTextPosition(offset: 0) }
  var endOfDocument: UITextPosition { 
    if let markedText = _markedText {
      return EditorTextPosition(offset: markedText.count)
    }
    return EditorTextPosition(offset: _shadowText.count)
  }

  var inputDelegate: (any UITextInputDelegate)?
  var tokenizer: any UITextInputTokenizer { UITextInputStringTokenizer(textInput: self) }

  func text(in range: UITextRange) -> String? {
    guard let editorRange = range as? EditorTextRange else { return nil }
    
    if let markedText = _markedText {
      let start = max(0, min(editorRange.startOffset, markedText.count))
      let end = max(start, min(editorRange.endOffset, markedText.count))
      if start >= markedText.count { return "" }
      let startIndex = markedText.index(markedText.startIndex, offsetBy: start)
      let endIndex = markedText.index(markedText.startIndex, offsetBy: end)
      return String(markedText[startIndex..<endIndex])
    }
    
    let text = _shadowText
    let start = max(0, min(editorRange.startOffset, text.count))
    let end = max(start, min(editorRange.endOffset, text.count))
    
    if start >= text.count {
      return ""
    }
    
    let startIndex = text.index(text.startIndex, offsetBy: start)
    let endIndex = text.index(text.startIndex, offsetBy: end)
    return String(text[startIndex..<endIndex])
  }

  func replace(_ range: UITextRange, withText text: String) {
    guard let editorRange = range as? EditorTextRange else { return }
    
    let shadowLen = _shadowText.count

    guard editorRange.startOffset >= 0 && editorRange.startOffset <= shadowLen else {
      onInsertText?(text)
      _shadowText = ""
      return
    }

    let deleteLength = shadowLen - editorRange.startOffset

    if deleteLength > 0 {
      onReplaceBackward?(deleteLength, text)
    } else {
      onInsertText?(text)
    }
    
    _shadowText = ""
  }

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
