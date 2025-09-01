import 'package:flutter/material.dart';
import 'package:typie/screens/editor/__generated__/editor_query.data.gql.dart';
import 'package:typie/screens/editor/schema.dart';
import 'package:typie/services/keyboard.dart';
import 'package:typie/widgets/webview.dart';

enum EditorMode { editor, note }

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

enum ConnectionStatus { connecting, connected, disconnected }

class EditorStateScope extends InheritedWidget {
  const EditorStateScope({
    required super.child,
    required this.data,
    required this.webViewController,
    required this.proseMirrorState,
    required this.characterCountState,
    required this.yjsState,
    required this.keyboardHeight,
    required this.isKeyboardVisible,
    required this.keyboardType,
    required this.mode,
    required this.bottomToolbarMode,
    required this.secondaryToolbarMode,
    required this.focusedElement,
    required this.connectionStatus,
    required this.isBottomSheetOpen,
    required this.scrollTop,
    super.key,
  });

  final ValueNotifier<GEditorScreen_QueryData?> data;
  final ValueNotifier<WebViewController?> webViewController;
  final ValueNotifier<ProseMirrorState?> proseMirrorState;
  final ValueNotifier<CharacterCountState?> characterCountState;
  final ValueNotifier<YJSState?> yjsState;
  final ValueNotifier<double> keyboardHeight;
  final ValueNotifier<bool> isKeyboardVisible;
  final ValueNotifier<KeyboardType> keyboardType;
  final ValueNotifier<EditorMode> mode;
  final ValueNotifier<BottomToolbarMode> bottomToolbarMode;
  final ValueNotifier<SecondaryToolbarMode> secondaryToolbarMode;
  final ValueNotifier<String?> focusedElement;
  final ValueNotifier<ConnectionStatus> connectionStatus;
  final ValueNotifier<bool> isBottomSheetOpen;
  final ValueNotifier<double> scrollTop;

  Future<void> command(String name, {Map<String, dynamic>? attrs}) async {
    await webViewController.value?.emitEvent('command', {'name': name, 'attrs': attrs});
  }

  static EditorStateScope of(BuildContext context) {
    final scope = context.getInheritedWidgetOfExactType<EditorStateScope>();
    return scope!;
  }

  @override
  bool updateShouldNotify(covariant EditorStateScope old) => false;
}
