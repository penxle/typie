import 'package:flutter/foundation.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/native/slate_reader.dart';
import 'package:typie/screens/native_editor/state/fonts.dart';
import 'package:typie/screens/native_editor/state/scroll_mode.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/table/models.dart';

class TrackedItemRange {
  const TrackedItemRange({required this.nodeId, required this.startOffset, required this.endOffset});

  final String nodeId;
  final int startOffset;
  final int endOffset;
}

class CurrentBlockOverlay {
  const CurrentBlockOverlay({
    required this.nodeId,
    required this.pageIdx,
    required this.x,
    required this.y,
    required this.width,
    required this.height,
  });

  final String nodeId;
  final int pageIdx;
  final double x;
  final double y;
  final double width;
  final double height;
}

class RemarkNodeContext {
  const RemarkNodeContext({required this.nodeType, required this.isTextblock, required this.nodeText});

  final String nodeType;
  final bool isTextblock;
  final String nodeText;
}

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

  bool restrictedText = false;
  bool restrictedBlob = false;
  void Function(String reason)? onEditBlocked;

  ScrollMode? pendingScrollMode;
  bool pendingScrollWaitForCursorUpdate = false;
  final ValueNotifier<List<TableOverlayInfo>> tableOverlays = ValueNotifier<List<TableOverlayInfo>>([]);
  List<InteractiveOverlayRaw> interactiveOverlays = const [];

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
  bool _disposed = false;
  bool get isDisposed => _disposed || editor.isDisposed;

  Map<int, Map<String, TrackedItemRange>> _trackedItemRanges = {};

  TrackedItemRange? trackedItemRange(int group, String id) {
    return _trackedItemRanges[group]?[id];
  }

  void setTrackedItemRanges(Map<int, Map<String, TrackedItemRange>> ranges) {
    _trackedItemRanges = ranges;
  }

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

  static const _insertionMessageTypes = {
    'input',
    'replaceBackward',
    'pasteHtml',
    'pasteHtmlAsText',
    'pasteText',
    'repasteAsText',
    'compositionStart',
    'compositionUpdate',
    'drop',
  };

  void dispatch(Map<String, dynamic> message) {
    if (isDisposed) {
      return;
    }
    if (restrictedText && _insertionMessageTypes.contains(message['type'])) {
      onEditBlocked?.call('restrictedText');
      return;
    }
    try {
      editor.dispatch(message);
    } on EditorException catch (err) {
      if (!_disposed) {
        debugPrint('EditorController dispatch skipped: $err');
      }
    }
  }

  void handleRepasteAsText() {
    if (!_state.repasteAsTextEnabled) {
      return;
    }
    dispatch({'type': 'repasteAsText'});
    scrollIntoView(mode: ScrollMode.typewriter);
    requestFocus();
  }

  void scrollIntoView({ScrollMode mode = ScrollMode.auto, bool waitForCursorUpdate = false}) {
    pendingScrollMode = mode;
    pendingScrollWaitForCursorUpdate = waitForCursorUpdate;
  }

  void clearPendingScroll() {
    pendingScrollMode = null;
    pendingScrollWaitForCursorUpdate = false;
  }

  final ValueNotifier<RemarkOverlayInfo?> remarkScrollTarget = ValueNotifier(null);
  final ValueNotifier<RemarkOverlayInfo?> remarkHighlightTarget = ValueNotifier(null);
  final ValueNotifier<Map<String, RemarkNodeContext>> remarkNodeContexts = ValueNotifier({});
  final ValueNotifier<double> sheetBottomInset = ValueNotifier(0);
  final ValueNotifier<CurrentBlockOverlay?> currentBlockOverlay = ValueNotifier(null);

  void scrollToRemark(RemarkOverlayInfo remark) {
    remarkScrollTarget.value = null;
    remarkHighlightTarget.value = null;
    remarkScrollTarget.value = remark;
    remarkHighlightTarget.value = remark;
  }

  void setFocused(bool focused) {
    if (_state.isFocused != focused) {
      if (!focused) {
        clearPendingScroll();
      }
      _state = _state.copyWith(isFocused: focused);
      dispatch({'type': 'setFocused', 'focused': focused});
      notifyListeners();
    }
  }

  void setTableOverlays(List<TableOverlayInfo> overlays) {
    tableOverlays.value = overlays;
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
    if (isDisposed) {
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
    _disposed = true;
    floatingContext.dispose();
    floatingNodeId.dispose();
    characterCounts.dispose();
    characterCountsVersion.dispose();
    tableOverlays.dispose();
    remarkScrollTarget.dispose();
    remarkHighlightTarget.dispose();
    remarkNodeContexts.dispose();
    sheetBottomInset.dispose();
    currentBlockOverlay.dispose();
    super.dispose();
  }
}
