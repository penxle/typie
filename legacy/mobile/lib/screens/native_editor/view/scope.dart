import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/controller/dnd_controller.dart';
import 'package:typie/screens/native_editor/controller/input.dart';
import 'package:typie/screens/native_editor/controller/ticker.dart';
import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/view/geometry.dart';
import 'package:typie/screens/native_editor/view/interaction/state.dart';
import 'package:typie/screens/native_editor/view/visible_area.dart';

class PresentedViewport {
  const PresentedViewport.base({required this.cursor, required this.renderVersion})
    : projectedScrollOffset = null,
      projectedMaxScrollExtent = null,
      projectedViewportHeight = null;

  const PresentedViewport.projected({
    required this.cursor,
    required this.renderVersion,
    required this.projectedScrollOffset,
    required this.projectedMaxScrollExtent,
    required this.projectedViewportHeight,
  });

  final CursorInfo? cursor;
  final Object? renderVersion;
  final double? projectedScrollOffset;
  final double? projectedMaxScrollExtent;
  final double? projectedViewportHeight;

  bool get hasProjectedMetrics =>
      projectedScrollOffset != null && projectedMaxScrollExtent != null && projectedViewportHeight != null;

  PresentedViewport clearProjection() {
    return PresentedViewport.base(cursor: cursor, renderVersion: renderVersion);
  }
}

class ContentScope extends InheritedWidget {
  const ContentScope({
    required super.child,
    required this.controller,
    required this.ticker,
    required this.verticalScrollController,
    required this.horizontalScrollController,
    required this.visibleArea,
    required this.viewportSize,
    required this.inputController,
    required this.longPressPosition,
    required this.handleDragPosition,
    required this.titleAreaHeight,
    required this.viewportTopInset,
    required this.title,
    required this.subtitle,
    required this.onTitleChanged,
    required this.onSubtitleChanged,
    required this.titleFocusNode,
    required this.subtitleFocusNode,
    required this.pendingScroll,
    required this.pendingScrollPageIdx,
    required this.visualSyncPageIdx,
    required this.presentedViewport,
    required this.dndController,
    required this.interactionState,
    required this.interactionSnapshot,
    required this.displayZoom,
    required this.renderZoom,
    required this.setZoom,
    super.key,
  });

  final EditorController controller;
  final EditorTicker ticker;
  final DndController dndController;
  final EditorInteractionState interactionState;
  final ValueListenable<InteractionSnapshot> interactionSnapshot;
  final ScrollController verticalScrollController;
  final ScrollController horizontalScrollController;
  final ValueListenable<VisibleEditorArea> visibleArea;
  final ValueNotifier<Size> viewportSize;
  final InputController inputController;

  final ValueNotifier<Offset?> longPressPosition;
  final ValueNotifier<Offset?> handleDragPosition;
  final ValueNotifier<double> titleAreaHeight;
  final ValueNotifier<double> viewportTopInset;
  final ValueNotifier<String> title;
  final ValueNotifier<String> subtitle;
  final ValueChanged<String> onTitleChanged;
  final ValueChanged<String> onSubtitleChanged;
  final FocusNode titleFocusNode;
  final FocusNode subtitleFocusNode;
  final ValueNotifier<VoidCallback?> pendingScroll;
  final ValueNotifier<int?> pendingScrollPageIdx;
  final ValueNotifier<int?> visualSyncPageIdx;
  final ValueNotifier<PresentedViewport> presentedViewport;
  final ValueNotifier<double> displayZoom;
  final ValueNotifier<double> renderZoom;
  final void Function(double zoom, {bool commitRender}) setZoom;

  NativeEditor get editor => controller.editor;

  VisibleEditorArea get visibleEditorArea => visibleArea.value;

  ContentGeometry get geometry {
    return ContentGeometry(
      layout: controller.state.layout!,
      pages: controller.state.pages,
      titleAreaHeight: titleAreaHeight.value,
      selection: controller.state.selection,
      zoom: displayZoom.value,
    );
  }

  static ContentScope of(BuildContext context) {
    return context.getInheritedWidgetOfExactType<ContentScope>()!;
  }

  @override
  bool updateShouldNotify(covariant ContentScope old) => false;
}
