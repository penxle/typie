import 'package:flutter/material.dart';
import 'package:typie/screens/editor/__generated__/editor_query.data.gql.dart';
import 'package:typie/screens/editor/schema.dart';
import 'package:typie/widgets/webview.dart';

enum SecondaryToolbarMode { hidden, text, textColor, fontFamily, fontSize, textAlign, lineHeight, letterSpacing }

enum BottomToolbarMode { hidden, insert }

class EditorStateScope extends InheritedWidget {
  const EditorStateScope({
    required super.child,
    required this.data,
    required this.webViewController,
    required this.proseMirrorState,
    required this.keyboardHeight,
    required this.isKeyboardVisible,
    required this.bottomToolbarMode,
    required this.secondaryToolbarMode,
    super.key,
  });

  final ValueNotifier<GEditorScreen_QueryData?> data;
  final ValueNotifier<WebViewController?> webViewController;
  final ValueNotifier<ProseMirrorState?> proseMirrorState;
  final ValueNotifier<double> keyboardHeight;
  final ValueNotifier<bool> isKeyboardVisible;
  final ValueNotifier<BottomToolbarMode> bottomToolbarMode;
  final ValueNotifier<SecondaryToolbarMode> secondaryToolbarMode;

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
