import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:typie/screens/native_editor/external/models.dart';

part 'state.freezed.dart';

enum SelectionHandleType { from, to }

@freezed
abstract class SearchScrollTarget with _$SearchScrollTarget {
  const factory SearchScrollTarget({
    required int pageIdx,
    required double x,
    required double y,
    required double width,
    required double height,
  }) = _SearchScrollTarget;
}

@freezed
abstract class SearchHighlightRect with _$SearchHighlightRect {
  const factory SearchHighlightRect({
    required double x,
    required double y,
    required double width,
    required double height,
  }) = _SearchHighlightRect;
}

@freezed
abstract class SearchOverlayInfo with _$SearchOverlayInfo {
  const factory SearchOverlayInfo({
    required int pageIdx,
    required bool isCurrent,
    required List<SearchHighlightRect> bounds,
  }) = _SearchOverlayInfo;
}

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

  bool isSamePosition(CursorInfo other) {
    return pageIdx == other.pageIdx && x == other.x && y == other.y;
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
abstract class SpellcheckOverlayBound with _$SpellcheckOverlayBound {
  const factory SpellcheckOverlayBound({
    required double x,
    required double y,
    required double width,
    required double height,
    required double ascent,
  }) = _SpellcheckOverlayBound;
}

@freezed
abstract class SpellcheckOverlayInfo with _$SpellcheckOverlayInfo {
  const factory SpellcheckOverlayInfo({
    required int pageIdx,
    required String id,
    required bool isActive,
    required List<SpellcheckOverlayBound> bounds,
  }) = _SpellcheckOverlayInfo;
}

@freezed
abstract class AiFeedbackOverlayBound with _$AiFeedbackOverlayBound {
  const factory AiFeedbackOverlayBound({
    required double x,
    required double y,
    required double width,
    required double height,
  }) = _AiFeedbackOverlayBound;
}

@freezed
abstract class AiFeedbackOverlayInfo with _$AiFeedbackOverlayInfo {
  const factory AiFeedbackOverlayInfo({
    required int pageIdx,
    required String id,
    required bool isActive,
    required List<AiFeedbackOverlayBound> bounds,
  }) = _AiFeedbackOverlayInfo;
}

@freezed
abstract class SearchState with _$SearchState {
  const factory SearchState({
    @Default(0) int totalCount,
    @Default(0) int currentIndex,
    SearchScrollTarget? scrollTarget,
    @Default([]) List<SearchOverlayInfo> overlays,
  }) = _SearchState;
}

@freezed
abstract class SpellcheckState with _$SpellcheckState {
  const factory SpellcheckState({
    @Default([]) List<SpellcheckOverlayInfo> overlays,
    String? activeErrorId,
    SpellcheckOverlayBound? scrollTarget,
    int? scrollTargetPageIdx,
  }) = _SpellcheckState;
}

@freezed
abstract class AiFeedbackState with _$AiFeedbackState {
  const factory AiFeedbackState({
    @Default([]) List<AiFeedbackOverlayInfo> overlays,
    String? activeItemId,
    AiFeedbackOverlayBound? scrollTarget,
    int? scrollTargetPageIdx,
  }) = _AiFeedbackState;
}

@freezed
abstract class PlaceholderInfo with _$PlaceholderInfo {
  const factory PlaceholderInfo({@Default(false) bool visible, double? x, double? y, double? width, double? height}) =
      _PlaceholderInfo;
}

@freezed
abstract class DropIndicatorInfo with _$DropIndicatorInfo {
  const factory DropIndicatorInfo({
    required int pageIdx,
    required double x,
    required double y,
    required double width,
    required double height,
  }) = _DropIndicatorInfo;

  const DropIndicatorInfo._();

  factory DropIndicatorInfo.fromMap(Map<String, dynamic> map) {
    final bounds = map['bounds'] as Map<String, dynamic>?;
    return DropIndicatorInfo(
      pageIdx: map['pageIdx'] as int? ?? 0,
      x: (bounds?['x'] as num?)?.toDouble() ?? 0,
      y: (bounds?['y'] as num?)?.toDouble() ?? 0,
      width: (bounds?['width'] as num?)?.toDouble() ?? 0,
      height: (bounds?['height'] as num?)?.toDouble() ?? 0,
    );
  }
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
    @Default(DocumentSettings()) DocumentSettings settings,
    SelectionHandleInfo? fromHandle,
    SelectionHandleInfo? toHandle,
    SelectionHandleType? draggingHandle,
    @Default(SearchState()) SearchState search,
    @Default(SpellcheckState()) SpellcheckState spellcheck,
    @Default(AiFeedbackState()) AiFeedbackState aiFeedback,
    @Default(PlaceholderInfo()) PlaceholderInfo placeholder,
    DropIndicatorInfo? dropIndicator,
    PasteOptionsInfo? pasteOptions,
  }) = _EditorState;

  const EditorState._();
}

@freezed
abstract class PasteOptionsInfo with _$PasteOptionsInfo {
  const factory PasteOptionsInfo({
    required String text,
    required Map<String, dynamic> from,
    required Map<String, dynamic> to,
  }) = _PasteOptionsInfo;
}
