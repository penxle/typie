import 'package:flutter/foundation.dart';
import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/external/models.dart';
import 'package:typie/screens/native_editor/state/fonts.dart';

part 'state.freezed.dart';

enum SelectionHandleType { from, to }

@freezed
abstract class SelectionHandleInfo with _$SelectionHandleInfo {
  const factory SelectionHandleInfo({
    required int pageIdx,
    required double x,
    required double y,
    required double height,
  }) = _SelectionHandleInfo;

  const SelectionHandleInfo._();

  factory SelectionHandleInfo.fromMap(Map<String, dynamic> map) {
    final bounds = map['bounds'] as Map<String, dynamic>?;
    return SelectionHandleInfo(
      pageIdx: map['pageIdx'] as int,
      x: (bounds?['x'] as num?)?.toDouble() ?? 0,
      y: (bounds?['y'] as num?)?.toDouble() ?? 0,
      height: (bounds?['height'] as num?)?.toDouble() ?? 0,
    );
  }
}

@freezed
abstract class CursorInfo with _$CursorInfo {
  const factory CursorInfo({
    required int pageIdx,
    required double x,
    required double y,
    required double height,
    required bool show,
    required bool scrollToCursor,
    required bool animate,
    required List<double> precedingCharWidths,
  }) = _CursorInfo;

  const CursorInfo._();

  factory CursorInfo.fromMap(Map<String, dynamic> map) {
    final bounds = map['bounds'] as Map<String, dynamic>?;
    return CursorInfo(
      pageIdx: map['pageIdx'] as int? ?? 0,
      x: (bounds?['x'] as num?)?.toDouble() ?? 0,
      y: (bounds?['y'] as num?)?.toDouble() ?? 0,
      height: (bounds?['height'] as num?)?.toDouble() ?? 0,
      show: map['show'] as bool? ?? false,
      scrollToCursor: map['scrollToCursor'] as bool? ?? false,
      animate: map['animate'] as bool? ?? false,
      precedingCharWidths: (map['precedingCharWidths'] as List?)?.map((e) => (e as num).toDouble()).toList() ?? [],
    );
  }
}

@freezed
abstract class LayoutModeInfo with _$LayoutModeInfo {
  const factory LayoutModeInfo.paginated({
    required double pageWidth,
    required double pageHeight,
    required double pageMarginTop,
    required double pageMarginBottom,
    required double pageMarginLeft,
    required double pageMarginRight,
  }) = PaginatedLayoutMode;

  const factory LayoutModeInfo.continuous({required double maxWidth}) = ContinuousLayoutMode;
}

@freezed
abstract class LayoutInfo with _$LayoutInfo {
  const factory LayoutInfo({
    required int pageCount,
    required bool isPaginated,
    required double pageWidth,
    required List<double> pageHeights,
    LayoutModeInfo? layoutMode,
  }) = _LayoutInfo;
}

@freezed
abstract class DocumentSettings with _$DocumentSettings {
  const factory DocumentSettings({@Default(1.0) double paragraphIndent, @Default(1.0) double blockGap}) =
      _DocumentSettings;
}

@freezed
abstract class EditorState with _$EditorState {
  const factory EditorState({
    LayoutInfo? layout,
    CursorInfo? cursor,
    @Default(false) bool isFocused,
    @Default(false) bool isSelecting,
    @Default([]) List<Map<String, dynamic>> uniformMarks,
    @Default([]) List<String> mixedMarks,
    @Default({}) Map<String, dynamic> selectionStats,
    @Default([]) List<ExternalElement> externalElements,
    Object? renderVersion,
    @Default(0) int fontLoadingCount,
    @Default(DocumentSettings()) DocumentSettings settings,
    SelectionHandleInfo? fromHandle,
    SelectionHandleInfo? toHandle,
    SelectionHandleType? draggingHandle,
  }) = _EditorState;

  const EditorState._();

  bool get isLoadingFonts => fontLoadingCount > 0;
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

  bool typewriterNeedsScroll = false;

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

  void updateState(EditorState Function(EditorState) updater) {
    _state = updater(_state);
    notifyListeners();
  }

  void dispatch(Map<String, dynamic> message) {
    if (!editor.isDisposed) {
      editor.dispatch(message);
    }
  }

  void setFocused(bool focused) {
    if (_state.isFocused != focused) {
      _state = _state.copyWith(isFocused: focused);
      dispatch({'type': 'setFocused', 'focused': focused});
      notifyListeners();
    }
  }

  void setSelecting(bool selecting) {
    if (_state.isSelecting != selecting) {
      _state = _state.copyWith(isSelecting: selecting);
      notifyListeners();
    }
  }

  void incrementFontLoading() {
    _state = _state.copyWith(fontLoadingCount: _state.fontLoadingCount + 1);
    notifyListeners();
  }

  void decrementFontLoading() {
    _state = _state.copyWith(fontLoadingCount: _state.fontLoadingCount - 1);
    notifyListeners();
  }
}
