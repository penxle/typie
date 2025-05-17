import 'package:flutter/material.dart';
import 'package:typie/screens/editor/schema.dart';
import 'package:typie/widgets/webview.dart';

class EditorStateScope extends InheritedWidget {
  const EditorStateScope({
    required super.child,
    required this.webViewController,
    required this.proseMirrorState,
    required this.keyboardHeight,
    required this.isKeyboardVisible,
    required this.selectedToolboxIdx,
    required this.selectedTextbarIdx,
    super.key,
  });

  final ValueNotifier<WebViewController?> webViewController;
  final ValueNotifier<ProseMirrorState?> proseMirrorState;
  final ValueNotifier<double> keyboardHeight;
  final ValueNotifier<bool> isKeyboardVisible;
  final ValueNotifier<int> selectedToolboxIdx;
  final ValueNotifier<int> selectedTextbarIdx;

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
