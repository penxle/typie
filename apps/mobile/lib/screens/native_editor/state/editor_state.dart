import 'package:flutter/foundation.dart';
import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/cursor.dart';
import 'package:typie/screens/native_editor/external/models.dart';
import 'package:typie/screens/native_editor/fonts.dart';

part 'editor_state.freezed.dart';

@freezed
abstract class LayoutInfo with _$LayoutInfo {
  const factory LayoutInfo({required int pageCount, required bool isPaginated, required List<double> pageHeights}) =
      _LayoutInfo;
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
  }) = _EditorState;
}

class EditorController extends ChangeNotifier {
  EditorController({required this.editor, required this.fontManager});

  final NativeEditor editor;
  final EditorFontManager? fontManager;

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
      notifyListeners();
    }
  }

  void setSelecting(bool selecting) {
    if (_state.isSelecting != selecting) {
      _state = _state.copyWith(isSelecting: selecting);
      notifyListeners();
    }
  }
}
