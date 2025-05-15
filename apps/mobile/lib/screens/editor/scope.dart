import 'package:flutter/material.dart';
import 'package:typie/screens/editor/schema.dart';
import 'package:typie/widgets/webview.dart';

class EditorStateScope extends InheritedWidget {
  EditorStateScope({required super.child, super.key});

  final ValueNotifier<WebViewController?> webViewController = ValueNotifier(null);
  final ValueNotifier<ProseMirrorState?> proseMirrorState = ValueNotifier(null);
  final ValueNotifier<double> keyboardHeight = ValueNotifier(0);
  final ValueNotifier<bool> isKeyboardVisible = ValueNotifier(false);
  final ValueNotifier<int> selectedToolboxIdx = ValueNotifier(-1);
  final ValueNotifier<int> selectedTextbarIdx = ValueNotifier(-1);

  Future<void> command(String name, {Map<String, dynamic>? attrs}) async {
    await webViewController.value?.emitEvent('command', {'name': name, 'attrs': attrs});
  }

  static EditorStateScope of(BuildContext context) {
    final scope = context.dependOnInheritedWidgetOfExactType<EditorStateScope>();
    return scope!;
  }

  @override
  bool updateShouldNotify(covariant EditorStateScope old) => false;
}
