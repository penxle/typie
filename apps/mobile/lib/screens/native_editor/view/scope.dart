import 'package:flutter/widgets.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/controller/dnd_controller.dart';
import 'package:typie/screens/native_editor/controller/input.dart';
import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/screens/native_editor/view/geometry.dart';

class ContentScope extends InheritedWidget {
  const ContentScope({
    required super.child,
    required this.controller,
    required this.verticalScrollController,
    required this.horizontalScrollController,
    required this.inputController,
    required this.isLongPressing,
    required this.longPressPosition,
    required this.handleDragPosition,
    required this.titleAreaHeight,
    required this.title,
    required this.subtitle,
    required this.onTitleChanged,
    required this.onSubtitleChanged,
    required this.titleFocusNode,
    required this.subtitleFocusNode,
    required this.pendingScroll,
    required this.dndController,
    super.key,
  });

  final EditorController controller;
  final DndController dndController;
  final ScrollController verticalScrollController;
  final ScrollController horizontalScrollController;
  final InputController inputController;

  final ValueNotifier<bool> isLongPressing;
  final ValueNotifier<Offset?> longPressPosition;
  final ValueNotifier<Offset?> handleDragPosition;
  final ValueNotifier<double> titleAreaHeight;
  final ValueNotifier<String> title;
  final ValueNotifier<String> subtitle;
  final ValueChanged<String> onTitleChanged;
  final ValueChanged<String> onSubtitleChanged;
  final FocusNode titleFocusNode;
  final FocusNode subtitleFocusNode;
  final ValueNotifier<VoidCallback?> pendingScroll;

  NativeEditor get editor => controller.editor;

  ContentGeometry get geometry {
    return ContentGeometry(
      layout: controller.state.layout!,
      pages: controller.state.pages,
      titleAreaHeight: titleAreaHeight.value,
    );
  }

  static ContentScope of(BuildContext context) {
    return context.getInheritedWidgetOfExactType<ContentScope>()!;
  }

  @override
  bool updateShouldNotify(covariant ContentScope old) => false;
}
