import 'package:flutter/material.dart';
import 'package:typie/screens/native_editor/controller/upload.dart';
import 'package:typie/screens/native_editor/external/models.dart';
import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/services/keyboard.dart';

enum SecondaryToolbarMode {
  hidden,
  text,
  textColor,
  textBackgroundColor,
  fontFamily,
  fontWeight,
  fontSize,
  textAlign,
  lineHeight,
  letterSpacing,
}

enum BottomToolbarMode { hidden, insert, horizontalRule, blockquote }

class NativeEditorToolbarScope extends InheritedWidget {
  const NativeEditorToolbarScope({
    required super.child,
    required this.controller,
    required this.keyboardHeight,
    required this.isKeyboardVisible,
    required this.keyboardType,
    required this.isEditorFocused,
    required this.bottomToolbarMode,
    required this.secondaryToolbarMode,
    required this.attrs,
    required this.externalElements,
    required this.uploadManager,
    required this.dispatch,
    required this.requestFocus,
    required this.clearFocus,
    required this.dismissKeyboard,
    required this.commitComposing,
    super.key,
  });

  final EditorController controller;

  final ValueNotifier<double> keyboardHeight;
  final ValueNotifier<bool> isKeyboardVisible;
  final ValueNotifier<KeyboardType> keyboardType;
  final ValueNotifier<bool> isEditorFocused;
  final ValueNotifier<BottomToolbarMode> bottomToolbarMode;
  final ValueNotifier<SecondaryToolbarMode> secondaryToolbarMode;

  final ValueNotifier<List<Map<String, dynamic>>> attrs;

  final ValueNotifier<List<ExternalElement>> externalElements;
  final UploadManager uploadManager;

  final void Function(Map<String, dynamic> message) dispatch;
  final void Function() requestFocus;
  final void Function() clearFocus;
  final void Function() dismissKeyboard;
  final void Function() commitComposing;

  static NativeEditorToolbarScope of(BuildContext context) {
    final scope = context.getInheritedWidgetOfExactType<NativeEditorToolbarScope>();
    return scope!;
  }

  @override
  bool updateShouldNotify(covariant NativeEditorToolbarScope old) => false;
}

Map<String, dynamic>? findAttr(List<Map<String, dynamic>> attrs, String type) {
  for (final a in attrs) {
    if (a['type'] == type) return a;
  }
  return null;
}
