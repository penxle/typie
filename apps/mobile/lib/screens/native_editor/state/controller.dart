import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/state/fonts.dart';
import 'package:typie/screens/native_editor/state/scroll_mode.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/table/models.dart';

class EditorController extends ChangeNotifier {
  EditorController({
    required this.editor,
    required this.fontManager,
    this.onDocChanged,
    this.onExitedDocumentStart,
    this.onSelectionChanged,
    this.onEditorReady,
  });

  final NativeEditor editor;
  final FontManager? fontManager;
  final void Function()? onDocChanged;
  final void Function()? onExitedDocumentStart;
  final void Function(Map<String, dynamic> anchor, Map<String, dynamic> head)? onSelectionChanged;
  final void Function()? onEditorReady;

  ScrollMode? pendingScrollMode;
  final ValueNotifier<List<TableOverlayInfo>> tableOverlays = ValueNotifier<List<TableOverlayInfo>>([]);

  VoidCallback? _clearFocusCallback;
  VoidCallback? _requestFocusCallback;

  void setClearFocusCallback(VoidCallback callback) {
    _clearFocusCallback = callback;
  }

  void setRequestFocusCallback(VoidCallback callback) {
    _requestFocusCallback = callback;
  }

  void clearFocus() {
    _clearFocusCallback?.call();
  }

  void requestFocus() {
    _requestFocusCallback?.call();
  }

  EditorState _state = const EditorState();
  EditorState get state => _state;
  final ValueNotifier<String?> floatingContext = ValueNotifier(null);
  final ValueNotifier<String?> floatingNodeId = ValueNotifier(null);
  final ValueNotifier<NativeEditorCharacterCounts?> characterCounts = ValueNotifier(null);
  final ValueNotifier<int> characterCountsVersion = ValueNotifier(0);

  bool _isBatching = false;
  bool _needsNotify = false;

  void beginBatchUpdate() {
    _isBatching = true;
    _needsNotify = false;
  }

  void endBatchUpdate() {
    _isBatching = false;
    if (_needsNotify) {
      _needsNotify = false;
      notifyListeners();
    }
  }

  void updateState(EditorState Function(EditorState) updater) {
    _state = updater(_state);
    if (_isBatching) {
      _needsNotify = true;
    } else {
      notifyListeners();
    }
  }

  void dispatch(Map<String, dynamic> message) {
    if (!editor.isDisposed) {
      editor.dispatch(message);
    }
  }

  void scrollIntoView({ScrollMode mode = ScrollMode.auto}) {
    pendingScrollMode = mode;
  }

  void setFocused(bool focused) {
    if (_state.isFocused != focused) {
      _state = _state.copyWith(isFocused: focused);
      dispatch({'type': 'setFocused', 'focused': focused});
      notifyListeners();
    }
  }

  void setTableOverlays(List<TableOverlayInfo> overlays) {
    tableOverlays.value = overlays;
  }

  void setSelecting(bool selecting) {
    if (_state.isSelecting != selecting) {
      _state = _state.copyWith(isSelecting: selecting);
      if (_isBatching) {
        _needsNotify = true;
      } else {
        notifyListeners();
      }
    }
  }

  final List<Completer<void>> _tickCompleters = [];

  Future<void> waitForNextTick() {
    final completer = Completer<void>();
    _tickCompleters.add(completer);
    return completer.future;
  }

  void notifyTick() {
    for (final completer in _tickCompleters) {
      completer.complete();
    }
    _tickCompleters.clear();
  }

  void setFloatingSelection({required String? context, required String? nodeId}) {
    if (floatingContext.value != context) {
      floatingContext.value = context;
    }
    if (floatingNodeId.value != nodeId) {
      floatingNodeId.value = nodeId;
    }
  }

  void refreshCharacterCounts() {
    if (editor.isDisposed) {
      return;
    }
    try {
      characterCounts.value = editor.getCharacterCounts();
    } on EditorException {
      // ignore
    }
  }

  void markCharacterCountsDirty() {
    characterCountsVersion.value++;
  }

  @override
  void dispose() {
    floatingContext.dispose();
    floatingNodeId.dispose();
    characterCounts.dispose();
    characterCountsVersion.dispose();
    tableOverlays.dispose();
    super.dispose();
  }
}
