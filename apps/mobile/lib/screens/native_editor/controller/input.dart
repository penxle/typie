import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/controller/clipboard.dart';
import 'package:typie/screens/native_editor/state/scroll_mode.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/view/input.dart';

class InputController {
  InputController({
    required this.inputKey,
    required void Function(Map<String, dynamic>) dispatch,
    required this.editor,
    required this.onFocusChanged,
    required this.scrollIntoView,
    required ValueGetter<BottomToolbarMode> getBottomToolbarMode,
    required ValueGetter<EditorSelection?> getEditorSelection,
    this.onInputAttempt,
  }) : _rawDispatch = dispatch,
       _getBottomToolbarMode = getBottomToolbarMode,
       _getEditorSelection = getEditorSelection;

  final GlobalKey<EditorTextInputState> inputKey;
  final void Function(Map<String, dynamic>) _rawDispatch;
  void Function(Map<String, dynamic>)? onDispatchRecorded;

  void dispatch(Map<String, dynamic> message) {
    _rawDispatch(message);
    onDispatchRecorded?.call(message);
  }

  final NativeEditor editor;
  final void Function(bool focused) onFocusChanged;
  final void Function({ScrollMode mode}) scrollIntoView;
  final ValueGetter<BottomToolbarMode> _getBottomToolbarMode;
  final ValueGetter<EditorSelection?> _getEditorSelection;
  final VoidCallback? onInputAttempt;

  bool _isActive = false;
  bool get isActive => _isActive;
  bool _pendingFocus = false;
  bool _inputReady = false;

  DateTime? _deleteStartTime;
  DateTime? _lastDeleteSignal;

  String? lastNodeId;

  void onInputReady() {
    _inputReady = true;
    if (_pendingFocus) {
      _pendingFocus = false;
      inputKey.currentState?.activateInput();
    }
  }

  void openInput() {
    if (!_isActive) {
      _isActive = true;
      onFocusChanged(true);
    }
    if (_inputReady) {
      inputKey.currentState?.activateInput();
    } else {
      _pendingFocus = true;
    }
  }

  void requestFocus() {
    _isActive = true;
    onFocusChanged(true);
    if (_inputReady) {
      inputKey.currentState?.activateInput();
    } else {
      _pendingFocus = true;
    }
  }

  void clearFocus() {
    if (!_isActive) {
      return;
    }
    _pendingFocus = false;
    invalidate();
    _isActive = false;
    onFocusChanged(false);
    inputKey.currentState?.deactivateInput();
  }

  void dismissKeyboard() {
    invalidate();
    inputKey.currentState?.deactivateInput();
  }

  void updateCursor(double x, double y, double height) {
    inputKey.currentState?.updateCursor(x, y, height);
  }

  void reconcile() {
    final selection = _getEditorSelection();
    if (selection != null) {
      final anchor = selection.range['anchor'] as Map<String, dynamic>;
      final nodeId = anchor['nodeId'] as String;
      final cursorOffset = anchor['offset'] as int;
      var precedingText = selection.precedingText ?? '';
      var followingText = selection.followingText ?? '';

      if (nodeId != lastNodeId) {
        if (lastNodeId != null) {
          inputKey.currentState?.invalidate();
        }
        lastNodeId = nodeId;
      }

      if (!selection.collapsed) {
        inputKey.currentState?.invalidate();
        precedingText = '';
        followingText = '';
      }

      final reconciled = inputKey.currentState?.reconcile(nodeId, cursorOffset, precedingText, followingText);
      if (reconciled ?? false) {
        dispatch({'type': 'commitPreedit'});
      }
    }
  }

  void invalidate() {
    inputKey.currentState?.invalidate();
    reconcile();
  }

  void onInsertText(String text) {
    onInputAttempt?.call();
    _deleteStartTime = null;
    dispatch({'type': 'input', 'text': text});
    scrollIntoView(mode: ScrollMode.typewriter);
  }

  void onDeleteBackward({int length = 1}) {
    onInputAttempt?.call();
    final now = DateTime.now();
    final lastSignal = _lastDeleteSignal;
    _lastDeleteSignal = now;

    final isRepeating = lastSignal != null && now.difference(lastSignal).inMilliseconds < 500;

    if (!isRepeating) {
      _deleteStartTime = null;
    }

    _deleteStartTime ??= now;
    final duration = now.difference(_deleteStartTime!).inMilliseconds / 1000.0;

    if (duration > 3.0) {
      dispatch({'type': 'deleteSentenceBackward'});
    } else if (duration > 1.5) {
      dispatch({'type': 'deleteWordBackward'});
    } else {
      dispatch({'type': 'deleteBackward', if (length > 1) 'length': length});
    }

    scrollIntoView(mode: ScrollMode.typewriter);
  }

  void compositionUpdate(String text, {int replaceLength = 0}) {
    onInputAttempt?.call();
    dispatch({'type': 'compositionUpdate', 'text': text, if (replaceLength > 0) 'replaceLength': replaceLength});
    scrollIntoView(mode: ScrollMode.typewriter);
  }

  void commitPreedit() {
    dispatch({'type': 'commitPreedit'});
    scrollIntoView(mode: ScrollMode.typewriter);
  }

  void compositionEnd() {
    dispatch({'type': 'compositionEnd'});
    scrollIntoView(mode: ScrollMode.typewriter);
  }

  void insertNewline() {
    onInputAttempt?.call();
    dispatch({'type': 'insertNewline'});
    scrollIntoView(mode: ScrollMode.typewriter);
  }

  void navigate(String direction) {
    dispatch({'type': 'navigate', 'direction': direction, 'extend': false});
    scrollIntoView(mode: ScrollMode.typewriter);
  }

  void onShortcut(String action) {
    onInputAttempt?.call();
    if (action == 'copy') {
      unawaited(EditorClipboard().copy(editor));
    } else if (action == 'cut') {
      unawaited(
        EditorClipboard().cut(editor, (msg) {
          dispatch(msg);
          scrollIntoView(mode: ScrollMode.typewriter);
        }),
      );
    } else if (action == 'paste') {
      if (onPasteHandler != null) {
        unawaited(onPasteHandler!());
      } else {
        unawaited(
          EditorClipboard().getPastePayload().then((payload) {
            if (payload != null) {
              dispatch(payload);
              scrollIntoView(mode: ScrollMode.typewriter);
            }
          }),
        );
      }
    } else if (action == 'toggleBold') {
      dispatch({
        'type': 'toggleStyle',
        'style': {'type': 'bold'},
      });
      scrollIntoView();
    } else if (action == 'toggleItalic') {
      dispatch({
        'type': 'toggleStyle',
        'style': {'type': 'italic'},
      });
      scrollIntoView();
    } else if (action == 'toggleUnderline') {
      dispatch({
        'type': 'toggleStyle',
        'style': {'type': 'underline'},
      });
      scrollIntoView();
    } else if (action == 'toggleStrikethrough') {
      dispatch({
        'type': 'toggleStyle',
        'style': {'type': 'strikethrough'},
      });
      scrollIntoView();
    } else if (action == 'selectAll') {
      dispatch({'type': action});
    } else {
      dispatch({'type': action});
      scrollIntoView();
    }
  }

  Future<void> Function()? onPasteHandler;

  VoidCallback? floatingCursorBeginHandler;
  void Function(double dx, double dy)? floatingCursorUpdateHandler;
  VoidCallback? floatingCursorEndHandler;

  void onFloatingCursorBegin() {
    invalidate();
    floatingCursorBeginHandler?.call();
  }

  void onFloatingCursorUpdate(double dx, double dy) {
    floatingCursorUpdateHandler?.call(dx, dy);
  }

  void onFloatingCursorEnd() {
    floatingCursorEndHandler?.call();
  }

  void onFocusLost() {
    if (_getBottomToolbarMode() != BottomToolbarMode.hidden) {
      return;
    }
    clearFocus();
  }

  void onReplaceBackward(int length, String text) {
    onInputAttempt?.call();
    dispatch({'type': 'replaceBackward', 'length': length, 'text': text});
    scrollIntoView(mode: ScrollMode.typewriter);
  }
}
