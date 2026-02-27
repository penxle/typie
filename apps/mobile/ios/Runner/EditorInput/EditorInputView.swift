import Flutter
import UIKit

private final class EditorBridgeTextView: UITextView {
  var onBeginFloatingCursor: ((CGPoint) -> Void)?
  var onUpdateFloatingCursor: ((CGPoint) -> Void)?
  var onEndFloatingCursor: (() -> Void)?
  var onAdjustTextPosition: ((Int) -> Void)?
  var onShortcutAction: ((String) -> Void)?

  override var undoManager: UndoManager? {
    nil
  }

  override func beginFloatingCursor(at point: CGPoint) {
    onBeginFloatingCursor?(point)
  }

  override func updateFloatingCursor(at point: CGPoint) {
    onUpdateFloatingCursor?(point)
  }

  override func endFloatingCursor() {
    onEndFloatingCursor?()
  }

  @objc(adjustTextPositionByCharacterOffset:)
  func adjustTextPositionByCharacterOffset(_ offset: Int) {
    onAdjustTextPosition?(offset)
  }

  override func copy(_ sender: Any?) {
    onShortcutAction?("copy")
  }

  override func cut(_ sender: Any?) {
    onShortcutAction?("cut")
  }

  override func paste(_ sender: Any?) {
    onShortcutAction?("paste")
  }

  override func selectAll(_ sender: Any?) {
    onShortcutAction?("selectAll")
  }

  @objc func undo(_ sender: Any?) {
    onShortcutAction?("undo")
  }

  @objc func redo(_ sender: Any?) {
    onShortcutAction?("redo")
  }

  @objc private func _handleUndoShortcut(_ sender: Any?) {
    onShortcutAction?("undo")
  }

  @objc private func _handleRedoShortcut(_ sender: Any?) {
    onShortcutAction?("redo")
  }
}

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

    inputView.onNavigate = { [weak self] direction, extend in
      self?.channel.invokeMethod("navigate", arguments: ["direction": direction, "extend": extend])
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
      case "setKeyboardAppearance":
        let args = call.arguments as? [String: Any]
        self.inputView.setKeyboardAppearance(args?["appearance"] as? String)
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

class EditorTextInputView: UIView, UITextInput, UITextViewDelegate {
  private static let cursorSentinelOffset: Int = 10000
  private static let maxShadowTextLength: Int = 256
  private static let navigationLeftPadding: Int = 64
  private static let navigationRightPadding: Int = 64
  private static let shadowFillUnit: String = " "

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
  var onNavigate: ((String, Bool) -> Void)?

  private var _markedText: String?
  private var _cursor: Int = EditorTextInputView.cursorSentinelOffset
  private var _isDeactivating: Bool = false
  private var _shadowText: String = ""
  private var _pendingSelectionRange: NSRange?
  private var _inputTextView: EditorBridgeTextView?
  private var _isSyncingInputTextViewState: Bool = false
  private var _precedingCharWidths: [Double] = []
  private static weak var _activeInputView: EditorTextInputView?
  private var _pendingTextEdit: (range: NSRange, text: String, beforeText: String)?
  private var _suppressSelectionNavigateOnce: Bool = false
  private var _lastSelectionTextLength: Int?
  private var _isNavigationShadowMode: Bool = false

  private var cursorX: Double = 0
  private var cursorY: Double = 0
  private var cursorHeight: Double = 20
  private var _floatingCursorStart: CGPoint?

  private var _textLengthForSelection: Int {
    return Swift.max(_shadowText.count, _markedText?.count ?? 0)
  }

  private func _clampOffset(_ offset: Int, max: Int) -> Int {
    return Swift.max(0, Swift.min(offset, max))
  }

  private func _consumeSelectionRange(shadowLength: Int) -> (start: Int, end: Int) {
    let range = _pendingSelectionRange
    _pendingSelectionRange = nil
    let collapsed = (shadowLength, shadowLength)
    guard let range else { return collapsed }
    let start = _clampOffset(range.location, max: shadowLength)
    let end = _clampOffset(range.location + range.length, max: shadowLength)
    return (Swift.min(start, end), Swift.max(start, end))
  }

  private func _setupInputTextViewIfNeeded() {
    guard _inputTextView == nil else { return }

    let textView = EditorBridgeTextView(frame: bounds)
    textView.delegate = self
    textView.backgroundColor = .clear
    textView.textColor = .clear
    textView.tintColor = .clear
    textView.isEditable = true
    textView.isSelectable = true
    textView.isScrollEnabled = false
    textView.autocorrectionType = .no
    textView.autocapitalizationType = .none
    textView.spellCheckingType = .no
    textView.smartQuotesType = .no
    textView.smartDashesType = .no
    textView.smartInsertDeleteType = .no
    textView.keyboardAppearance = keyboardAppearance
    textView.textContainerInset = .zero
    textView.textContainer.lineFragmentPadding = 0
    textView.inputAssistantItem.leadingBarButtonGroups = []
    textView.inputAssistantItem.trailingBarButtonGroups = []
    textView.alpha = 0.01
    textView.onBeginFloatingCursor = { [weak self] point in
      self?.beginFloatingCursor(at: point)
    }
    textView.onUpdateFloatingCursor = { [weak self] point in
      self?.updateFloatingCursor(at: point)
    }
    textView.onEndFloatingCursor = { [weak self] in
      self?.endFloatingCursor()
    }
    textView.onAdjustTextPosition = { [weak self] offset in
      self?.adjustTextPosition(byCharacterOffset: offset)
    }
    textView.onShortcutAction = { [weak self] action in
      self?._handleShortcutAction(action)
    }

    addSubview(textView)
    _inputTextView = textView
    _syncInputTextViewState()
  }

  private func _reanchorNavigationShadow(precedingCount: Int? = nil) {
    guard _markedText == nil else { return }

    let leftCount = max(precedingCount ?? _precedingCharWidths.count, 0)
    let minimumLength = max(
      Self.navigationLeftPadding + 1 + Self.navigationRightPadding,
      leftCount + Self.navigationRightPadding
    )
    if _shadowText.count < minimumLength {
      _shadowText = String(repeating: Self.shadowFillUnit, count: minimumLength)
    } else {
      _normalizeShadowTextContent()
    }

    let maxAnchor = _clampOffset(_shadowText.count - Self.navigationRightPadding, max: _shadowText.count)
    let anchor = _clampOffset(max(Self.navigationLeftPadding, leftCount), max: maxAnchor)
    _isNavigationShadowMode = true
    _pendingSelectionRange = NSRange(location: anchor, length: 0)
    _cursor = Self.cursorSentinelOffset + anchor
    _syncInputTextViewState(selectedOffset: anchor)
  }

  private func _prepareNavigationShadow(
    precedingCount: Int? = nil,
    resetSelectionTracking: Bool = false
  ) {
    if resetSelectionTracking {
      _suppressSelectionNavigateOnce = false
      _lastSelectionTextLength = nil
    }
    _reanchorNavigationShadow(precedingCount: precedingCount)
  }

  private func _syncInputTextViewState(selectedOffset: Int? = nil) {
    guard let textView = _inputTextView else { return }
    _isSyncingInputTextViewState = true
    textView.text = _shadowText
    let selectionLocation = _clampOffset(selectedOffset ?? _shadowText.count, max: _shadowText.count)
    textView.selectedRange = NSRange(location: selectionLocation, length: 0)
    _isSyncingInputTextViewState = false
  }

  private func _clearMarkedText(notify: (() -> Void)? = nil) {
    guard _markedText != nil else { return }
    _markedText = nil
    notify?()
  }

  private func _resetShadowState(syncInputTextView: Bool) {
    _pendingSelectionRange = nil
    _pendingTextEdit = nil
    _lastSelectionTextLength = nil
    _suppressSelectionNavigateOnce = false
    _isNavigationShadowMode = false
    _shadowText = ""
    _cursor = Self.cursorSentinelOffset
    if syncInputTextView {
      _syncInputTextViewState(selectedOffset: 0)
    }
  }

  private func _truncateShadowTextIfNeeded(syncInputTextView: Bool) {
    let maxLen = Self.maxShadowTextLength
    let shadowLen = _shadowText.count
    guard shadowLen > maxLen else { return }

    let droppedCount = shadowLen - maxLen
    _shadowText = String(_shadowText.suffix(maxLen))

    if let range = _pendingSelectionRange {
      let rawStart = range.location - droppedCount
      let rawEnd = (range.location + range.length) - droppedCount
      let clampedStart = _clampOffset(rawStart, max: _shadowText.count)
      let clampedEnd = _clampOffset(rawEnd, max: _shadowText.count)
      let normalizedStart = Swift.min(clampedStart, clampedEnd)
      let normalizedEnd = Swift.max(clampedStart, clampedEnd)
      _pendingSelectionRange = NSRange(location: normalizedStart, length: normalizedEnd - normalizedStart)
      _cursor = Self.cursorSentinelOffset + normalizedEnd
    } else {
      _cursor = Self.cursorSentinelOffset + _shadowText.count
    }

    if syncInputTextView {
      let selectedOffset = _pendingSelectionRange.map { $0.location + $0.length } ?? _shadowText.count
      _syncInputTextViewState(selectedOffset: selectedOffset)
    }
  }

  private func _resignInputResponder() {
    if let bridge = _inputTextView, bridge.isFirstResponder {
      _ = bridge.resignFirstResponder()

      if isFirstResponder {
        _ = resignFirstResponder()
      }
    } else {
      _ = resignFirstResponder()
    }
  }

  @discardableResult
  private func _becomeInputResponder() -> Bool {
    _setupInputTextViewIfNeeded()
    _syncInputTextViewState()
    guard let inputTextView = _inputTextView else { return false }
    return inputTextView.becomeFirstResponder()
  }


  override init(frame: CGRect) {
    super.init(frame: frame)
    backgroundColor = .clear
    _setupInputTextViewIfNeeded()
  }

  deinit {
    if Self._activeInputView === self {
      Self._activeInputView = nil
    }
    _keyRepeatTimer?.invalidate()
  }

  override func hitTest(_ point: CGPoint, with event: UIEvent?) -> UIView? {
    return nil
  }

  override func layoutSubviews() {
    super.layoutSubviews()
    if let textView = _inputTextView {
      textView.frame = bounds.isEmpty ? CGRect(x: 0, y: 0, width: 1, height: 1) : bounds
    }
  }

  required init?(coder: NSCoder) {
    fatalError("init(coder:) has not been implemented")
  }

  func setKeyboardAppearance(_ appearance: String?) {
    switch appearance {
    case "light":
      keyboardAppearance = .light
    case "dark":
      keyboardAppearance = .dark
    default:
      keyboardAppearance = .default
    }

    _inputTextView?.keyboardAppearance = keyboardAppearance

    if isFirstResponder || (_inputTextView?.isFirstResponder ?? false) {
      reloadInputViews()
      _inputTextView?.reloadInputViews()
    }
  }

  func activate() {
    DispatchQueue.main.async { [weak self] in
      guard let self = self else { return }

      if let active = Self._activeInputView,
         active !== self,
         let activeWindow = active.window,
         let selfWindow = self.window,
         activeWindow === selfWindow {
        active._isDeactivating = true
        active._resignInputResponder()
        active._isDeactivating = false
      }

      if self._becomeInputResponder() {
        self._prepareNavigationShadow(resetSelectionTracking: true)
        Self._activeInputView = self
        return
      }
      
      DispatchQueue.main.async { [weak self] in
        guard let self = self else { return }
        if self._becomeInputResponder() {
          self._prepareNavigationShadow(resetSelectionTracking: true)
          Self._activeInputView = self
        }
      }
    }
  }

  func deactivate() {
    DispatchQueue.main.async { [weak self] in
      self?._isDeactivating = true
      self?._resignInputResponder()
      self?._isDeactivating = false
    }
  }

  @discardableResult
  override func resignFirstResponder() -> Bool {
    if let bridge = _inputTextView, bridge.isFirstResponder {
      return bridge.resignFirstResponder()
    }

    let result = super.resignFirstResponder()
    if result {
      if Self._activeInputView === self {
        Self._activeInputView = nil
      }
      if !_isDeactivating {
        onFocusLost?()
      }
    }
    return result
  }

  func updateCursor(x: Double, y: Double, height: Double, precedingCharWidths: [Double]? = nil) {
    cursorX = x
    cursorY = y
    cursorHeight = height
    if let widths = precedingCharWidths {
      _precedingCharWidths = widths
      if _isNavigationShadowMode || _shadowText.isEmpty {
        _prepareNavigationShadow(precedingCount: widths.count)
      }
    }
  }

  func resetInputContext() {
    _clearMarkedText(notify: onUnmarkText)
    _resetShadowState(syncInputTextView: true)
    inputDelegate?.textWillChange(self)
    inputDelegate?.textDidChange(self)
  }

  override var canBecomeFirstResponder: Bool { true }
  
  override var undoManager: UndoManager? { nil }

  // MARK: - Floating Cursor (keyboard trackpad mode)

  func beginFloatingCursor(at point: CGPoint) {
    _clearMarkedText(notify: onUnmarkText)
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

  func adjustTextPosition(byCharacterOffset offset: Int) {
    guard offset != 0 else { return }
    _clearMarkedText(notify: onUnmarkText)
    let direction = offset > 0 ? "right" : "left"
    for _ in 0..<abs(offset) {
      onNavigate?(direction, false)
    }
  }

  private var _keyRepeatTimer: Timer?
  private var _currentArrowDirection: String?
  private var _currentExtend: Bool = false
  private static let keyRepeatInitialDelay: TimeInterval = 0.4
  private static let keyRepeatInterval: TimeInterval = 0.05

  private func startKeyRepeat(direction: String, extend: Bool) {
    _currentArrowDirection = direction
    _currentExtend = extend
    _keyRepeatTimer?.invalidate()

    onNavigate?(direction, extend)

    _keyRepeatTimer = Timer.scheduledTimer(withTimeInterval: Self.keyRepeatInitialDelay, repeats: false) { [weak self] initialTimer in
      initialTimer.invalidate()
      guard let self = self, let dir = self._currentArrowDirection else { return }

      self._keyRepeatTimer = Timer.scheduledTimer(withTimeInterval: Self.keyRepeatInterval, repeats: true) { [weak self] _ in
        guard let self = self, let dir = self._currentArrowDirection else { return }
        self.onNavigate?(dir, self._currentExtend)
      }
    }
  }

  private func stopKeyRepeat() {
    _keyRepeatTimer?.invalidate()
    _keyRepeatTimer = nil
    _currentArrowDirection = nil
  }

  override func pressesBegan(_ presses: Set<UIPress>, with event: UIPressesEvent?) {
    var handled = false
    for press in presses {
      if let (direction, extend) = getArrowKeyDirection(press) {
        _clearMarkedText(notify: onUnmarkText)
        startKeyRepeat(direction: direction, extend: extend)
        handled = true
        break
      }
    }
    if !handled {
      super.pressesBegan(presses, with: event)
    }
  }

  override func pressesEnded(_ presses: Set<UIPress>, with event: UIPressesEvent?) {
    var handled = false
    for press in presses {
      if let _ = getArrowKeyDirection(press) {
        stopKeyRepeat()
        handled = true
      }
    }
    if !handled {
      super.pressesEnded(presses, with: event)
    }
  }

  override func pressesCancelled(_ presses: Set<UIPress>, with event: UIPressesEvent?) {
    stopKeyRepeat()
    super.pressesCancelled(presses, with: event)
  }

  private func getArrowKeyDirection(_ press: UIPress) -> (String, Bool)? {
    guard let key = press.key else { return nil }
    
    let direction: String?
    switch key.keyCode {
    case .keyboardLeftArrow:
      if key.modifierFlags.contains(.command) {
        direction = "lineStart"
      } else if key.modifierFlags.contains(.alternate) {
        direction = "wordLeft"
      } else {
        direction = "left"
      }
    case .keyboardRightArrow:
      if key.modifierFlags.contains(.command) {
        direction = "lineEnd"
      } else if key.modifierFlags.contains(.alternate) {
        direction = "wordRight"
      } else {
        direction = "right"
      }
    case .keyboardUpArrow:
      if key.modifierFlags.contains(.command) {
        direction = "documentStart"
      } else if key.modifierFlags.contains(.alternate) {
        direction = "sentenceUp"
      } else {
        direction = "up"
      }
    case .keyboardDownArrow:
      if key.modifierFlags.contains(.command) {
        direction = "documentEnd"
      } else if key.modifierFlags.contains(.alternate) {
        direction = "sentenceDown"
      } else {
        direction = "down"
      }
    default:
      direction = nil
    }
    
    guard let dir = direction else { return nil }
    let extend = key.modifierFlags.contains(.shift)
    return (dir, extend)
  }

  // MARK: - UITextInputTraits

  var autocapitalizationType: UITextAutocapitalizationType = .none
  var autocorrectionType: UITextAutocorrectionType = .no
  var spellCheckingType: UITextSpellCheckingType = .no
  var smartQuotesType: UITextSmartQuotesType = .no
  var smartDashesType: UITextSmartDashesType = .no
  var smartInsertDeleteType: UITextSmartInsertDeleteType = .no
  var keyboardAppearance: UIKeyboardAppearance = .default


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
    ("c", .command, "copy"),
    ("x", .command, "cut"),
    ("v", .command, "paste"),
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
        _handleShortcutAction(def.action)
        return
      }
    }
  }

  private func _handleShortcutAction(_ action: String) {
    _clearMarkedText(notify: onUnmarkText)
    onShortcut?(action)

    _resetShadowState(syncInputTextView: true)
    inputDelegate?.textWillChange(self)
    inputDelegate?.selectionWillChange(self)
    inputDelegate?.textDidChange(self)
    inputDelegate?.selectionDidChange(self)
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

  private func _dispatchTextDelta(from oldText: String, to newText: String) {
    if oldText == newText {
      return
    }

    let oldChars = Array(oldText)
    let newChars = Array(newText)

    var prefix = 0
    while prefix < oldChars.count, prefix < newChars.count, oldChars[prefix] == newChars[prefix] {
      prefix += 1
    }

    var oldSuffix = oldChars.count
    var newSuffix = newChars.count
    while oldSuffix > prefix,
          newSuffix > prefix,
          oldChars[oldSuffix - 1] == newChars[newSuffix - 1] {
      oldSuffix -= 1
      newSuffix -= 1
    }

    let deleteLength = oldSuffix - prefix
    let insertedText = String(newChars[prefix..<newSuffix])

    if deleteLength == 0 {
      if !insertedText.isEmpty {
        onInsertText?(insertedText)
      }
      return
    }

    if insertedText.isEmpty {
      if deleteLength == 1 {
        onDeleteBackward?()
      } else {
        onReplaceBackward?(deleteLength, "")
      }
      return
    }

    onReplaceBackward?(deleteLength, insertedText)
  }

  private func _deleteLengthForUTF16Range(_ range: NSRange, in text: String) -> Int {
    let nsText = text as NSString
    let utf16Length = nsText.length
    let utf16Start = _clampOffset(range.location, max: utf16Length)
    let utf16End = _clampOffset(range.location + range.length, max: utf16Length)
    let clampedLength = Swift.max(0, utf16End - utf16Start)

    let deleted = nsText.substring(with: NSRange(location: utf16Start, length: clampedLength))
    return deleted.count
  }

  private func _dispatchTextEdit(range: NSRange, replacementText: String, previousText: String) {
    let deleteLength = _deleteLengthForUTF16Range(range, in: previousText)

    if replacementText.isEmpty {
      if deleteLength == 1 {
        onDeleteBackward?()
      } else if deleteLength > 1 {
        onReplaceBackward?(deleteLength, "")
      }
      return
    }

    if deleteLength == 0 {
      onInsertText?(replacementText)
    } else {
      onReplaceBackward?(deleteLength, replacementText)
    }
  }

  private func _appliesPendingTextEdit(_ pending: (range: NSRange, text: String, beforeText: String), to currentText: String) -> Bool {
    let before = pending.beforeText as NSString
    guard pending.range.location >= 0,
          pending.range.length >= 0,
          pending.range.location + pending.range.length <= before.length else {
      return false
    }

    let expected = before.replacingCharacters(in: pending.range, with: pending.text)
    return expected == currentText
  }

  private func _normalizeShadowTextContent() {
    guard !_shadowText.isEmpty else { return }
    _shadowText = String(repeating: Self.shadowFillUnit, count: _shadowText.count)
  }

  private func _repinAfterBoundaryBackspace() {
    _prepareNavigationShadow(precedingCount: max(_precedingCharWidths.count, 1))
  }

  private func _syncSelectionStateFromTextView(_ textView: UITextView, textLength: Int) {
    let start = _clampOffset(textView.selectedRange.location, max: textLength)
    let length = _clampOffset(textView.selectedRange.length, max: textLength - start)
    _pendingSelectionRange = NSRange(location: start, length: length)
    _cursor = Self.cursorSentinelOffset + start + length
  }

  private func _isWhitespaceSelection(_ range: NSRange, in text: String) -> Bool {
    guard range.length > 0 else { return false }

    let chars = Array(text)
    let start = _clampOffset(range.location, max: chars.count)
    let end = _clampOffset(range.location + range.length, max: chars.count)
    guard end > start else { return false }

    for idx in start..<end where !chars[idx].isWhitespace {
      return false
    }

    return true
  }

  private func _collapseSelectionIfNeeded(textLength: Int, currentText: String) -> Bool {
    guard !_isNavigationShadowMode else { return false }
    let range = _normalizedSelectionRange(_pendingSelectionRange, textLength: textLength)
    guard range.length > 0 else { return false }
    guard currentText.contains(where: { !$0.isWhitespace }) else { return false }
    guard _isWhitespaceSelection(range, in: currentText) else { return false }

    let collapseOffset = range.location + range.length
    _pendingSelectionRange = NSRange(location: collapseOffset, length: 0)
    _cursor = Self.cursorSentinelOffset + collapseOffset
    _syncInputTextViewState(selectedOffset: collapseOffset)
    return true
  }

  private func _normalizedSelectionRange(_ range: NSRange?, textLength: Int) -> NSRange {
    guard let range else {
      return NSRange(location: textLength, length: 0)
    }

    let start = _clampOffset(range.location, max: textLength)
    let end = _clampOffset(range.location + range.length, max: textLength)
    let normalizedStart = Swift.min(start, end)
    let normalizedEnd = Swift.max(start, end)
    return NSRange(location: normalizedStart, length: normalizedEnd - normalizedStart)
  }

  func textView(_ textView: UITextView, shouldChangeTextIn range: NSRange, replacementText text: String) -> Bool {
    guard !_isSyncingInputTextViewState else { return true }
    _pendingTextEdit = nil

    if _markedText != nil && text.isEmpty {
      _suppressSelectionNavigateOnce = true
    }

    if text.isEmpty, range.length == 0, range.location == 0, textView.markedTextRange == nil {
      onDeleteBackward?()
      _repinAfterBoundaryBackspace()
      return false
    }

    if text == "\n", textView.markedTextRange == nil {
      if isSoftKeyboardShiftActive {
        onShortcut?("insertHardBreak")
      } else {
        onPerformAction?("newline")
      }
      _resetShadowState(syncInputTextView: true)
      return false
    }

    if textView.markedTextRange == nil, _markedText == nil {
      _pendingTextEdit = (range: range, text: text, beforeText: textView.text ?? "")
    }

    return true
  }

  func textViewDidChange(_ textView: UITextView) {
    guard !_isSyncingInputTextViewState else { return }

    let currentText = textView.text ?? ""
    _syncSelectionStateFromTextView(textView, textLength: currentText.count)

    if let markedRange = textView.markedTextRange,
       let markedText = textView.text(in: markedRange),
       !markedText.isEmpty {
      _pendingTextEdit = nil
      _isNavigationShadowMode = false
      if _markedText != markedText {
        _markedText = markedText
        onSetMarkedText?(markedText)
      }
      return
    }

    let hadMarkedText = _markedText != nil
    _clearMarkedText(notify: onCancelMarkedText)
    if hadMarkedText {
      _suppressSelectionNavigateOnce = true
    }

    let previousText = _shadowText
    _shadowText = currentText
    _isNavigationShadowMode = false
    if let pending = _pendingTextEdit, _appliesPendingTextEdit(pending, to: currentText) {
      _dispatchTextEdit(range: pending.range, replacementText: pending.text, previousText: previousText)
    } else {
      _dispatchTextDelta(from: previousText, to: currentText)
    }
    _pendingTextEdit = nil
    _truncateShadowTextIfNeeded(syncInputTextView: true)
  }

  func textViewDidChangeSelection(_ textView: UITextView) {
    guard !_isSyncingInputTextViewState else { return }

    let currentText = textView.text ?? ""
    let textLength = currentText.count

    if let lastLength = _lastSelectionTextLength, lastLength != textLength {
      _lastSelectionTextLength = textLength
      _syncSelectionStateFromTextView(textView, textLength: textLength)
      return
    }
    _lastSelectionTextLength = textLength

    let previousRange = _normalizedSelectionRange(_pendingSelectionRange, textLength: textLength)

    _syncSelectionStateFromTextView(textView, textLength: textLength)

    if _markedText != nil {
      return
    }

    if _suppressSelectionNavigateOnce {
      _suppressSelectionNavigateOnce = false
      return
    }

    if _floatingCursorStart != nil {
      return
    }

    guard textView.markedTextRange == nil, currentText == _shadowText else { return }

    if _collapseSelectionIfNeeded(textLength: textLength, currentText: currentText) {
      return
    }

    let nextRange = _normalizedSelectionRange(_pendingSelectionRange, textLength: textLength)
    guard previousRange.length == 0, nextRange.length == 0 else { return }

    let delta = nextRange.location - previousRange.location
    guard delta != 0 else { return }

    let direction = delta > 0 ? "right" : "left"
    for _ in 0..<abs(delta) {
      onNavigate?(direction, false)
    }
    _normalizeShadowTextContent()

    let anchor: Int
    if textLength >= 2 {
      anchor = textLength / 2
    } else if textLength == 1 {
      anchor = direction == "right" ? 0 : 1
    } else {
      anchor = 0
    }

    _pendingSelectionRange = NSRange(location: anchor, length: 0)
    _cursor = Self.cursorSentinelOffset + anchor
    _syncInputTextViewState(selectedOffset: anchor)
  }

  func textViewDidEndEditing(_ textView: UITextView) {
    if Self._activeInputView === self {
      Self._activeInputView = nil
    }
    if !_isDeactivating {
      onFocusLost?()
    }
  }

  func insertText(_ text: String) {
    _clearMarkedText()

    if text == "\n" {
      if isSoftKeyboardShiftActive {
        onShortcut?("insertHardBreak")
      } else {
        onPerformAction?("newline")
      }
      _resetShadowState(syncInputTextView: false)
      return
    }

    let shadowLength = _shadowText.count
    let selection = _consumeSelectionRange(shadowLength: shadowLength)
    let replaceLength = selection.end - selection.start
    let replacesSuffix = replaceLength > 0 && selection.end == shadowLength

    let startIndex = _shadowText.index(_shadowText.startIndex, offsetBy: selection.start)
    let endIndex = _shadowText.index(_shadowText.startIndex, offsetBy: selection.end)
    _shadowText.replaceSubrange(startIndex..<endIndex, with: text)
    _truncateShadowTextIfNeeded(syncInputTextView: false)
    _cursor = Self.cursorSentinelOffset + _shadowText.count

    inputDelegate?.textWillChange(self)
    inputDelegate?.textDidChange(self)

    if replacesSuffix {
      onReplaceBackward?(replaceLength, text)
    } else {
      onInsertText?(text)
    }
  }

  func deleteBackward() {
    if _markedText != nil {
      _clearMarkedText(notify: onCancelMarkedText)
      return
    }

    let shadowLength = _shadowText.count
    var selection = _consumeSelectionRange(shadowLength: shadowLength)
    if selection.start == selection.end {
      if selection.end == 0 {
        _resetShadowState(syncInputTextView: false)
        onDeleteBackward?()
        return
      }
      selection = (selection.end - 1, selection.end)
    }

    let deleteLength = selection.end - selection.start
    let deletesSuffix = selection.end == shadowLength

    let startIndex = _shadowText.index(_shadowText.startIndex, offsetBy: selection.start)
    let endIndex = _shadowText.index(_shadowText.startIndex, offsetBy: selection.end)
    _shadowText.removeSubrange(startIndex..<endIndex)
    _cursor = Self.cursorSentinelOffset + _shadowText.count

    inputDelegate?.textWillChange(self)
    inputDelegate?.textDidChange(self)

    if deletesSuffix {
      if deleteLength == 1 {
        onDeleteBackward?()
      } else {
        onReplaceBackward?(deleteLength, "")
      }
    } else {
      onDeleteBackward?()
    }
  }

  // MARK: - UITextInput (Marked Text)

  var markedTextRange: UITextRange? {
    guard let markedText = _markedText else { return nil }
    return EditorTextRange(start: 0, end: markedText.count)
  }

  var markedTextStyle: [NSAttributedString.Key: Any]?

  func setMarkedText(_ markedText: String?, selectedRange: NSRange) {
    if let text = markedText, !text.isEmpty {
      _pendingSelectionRange = nil
      _markedText = text
      onSetMarkedText?(text)
    } else {
      _pendingSelectionRange = nil
      _clearMarkedText(notify: onCancelMarkedText)
    }
  }

  func unmarkText() {
    _pendingSelectionRange = nil
    _clearMarkedText(notify: onUnmarkText)
  }

  // MARK: - UITextInput (Selection)

  var selectedTextRange: UITextRange? {
    get {
      if let range = _pendingSelectionRange {
        return EditorTextRange(start: range.location, end: range.location + range.length)
      }
      if let markedText = _markedText {
        let pos = markedText.count
        return EditorTextRange(start: pos, end: pos)
      }
      let pos = _shadowText.isEmpty ? _cursor : _shadowText.count
      return EditorTextRange(start: pos, end: pos)
    }
    set {
      guard let editorRange = newValue as? EditorTextRange else { return }
      let maxLen = _shadowText.count
      let start = _clampOffset(editorRange.startOffset, max: maxLen)
      let end = _clampOffset(editorRange.endOffset, max: maxLen)
      let normalizedStart = Swift.min(start, end)
      let normalizedEnd = Swift.max(start, end)
      _pendingSelectionRange = NSRange(location: normalizedStart, length: normalizedEnd - normalizedStart)
      _cursor = Self.cursorSentinelOffset + normalizedEnd
      inputDelegate?.selectionWillChange(self)
      inputDelegate?.selectionDidChange(self)
    }
  }

  // MARK: - UITextInput (Text Geometry)

  func firstRect(for range: UITextRange) -> CGRect {
    guard let editorRange = range as? EditorTextRange else {
      return CGRect(x: cursorX, y: cursorY, width: 1, height: cursorHeight)
    }
    
    let shadowLen = _shadowText.count
    let rangeStart = max(0, min(editorRange.startOffset, shadowLen))
    let rangeEnd = max(rangeStart, min(editorRange.endOffset, shadowLen))
    
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
    let widthStart = min(rangeStart, availableWidths)
    let widthEnd = min(rangeEnd, availableWidths)
    if widthEnd > widthStart {
      for i in widthStart..<widthEnd {
        width += _precedingCharWidths[i]
      }
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
    return EditorTextPosition(offset: _shadowText.isEmpty ? _cursor : _shadowText.count)
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
    let maxLen = _textLengthForSelection
    if newOffset < 0 || newOffset > maxLen { return nil }
    return EditorTextPosition(offset: newOffset)
  }

  func position(from position: UITextPosition, in direction: UITextLayoutDirection, offset: Int) -> UITextPosition? {
    guard let pos = position as? EditorTextPosition else { return nil }
    let delta = (direction == .left || direction == .up) ? -offset : offset
    let newOffset = pos.offset + delta
    let maxLen = _textLengthForSelection
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
    return EditorTextPosition(offset: _textLengthForSelection)
  }

  func closestPosition(to point: CGPoint, within range: UITextRange) -> UITextPosition? {
    return EditorTextPosition(offset: _textLengthForSelection)
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
    let maxLen = _textLengthForSelection
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
