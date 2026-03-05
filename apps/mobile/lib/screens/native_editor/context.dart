import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/state/controller.dart';

class EditorContext {
  NativeEditor? editor;
  EditorController? controller;
  Uint8List? serverSnapshot;
  String? serverVersion;
  int serverGeneration = 0;
  VoidCallback? showInputRecordingSheet;

  final ValueNotifier<int> resetKey = ValueNotifier(0);

  void dispose() {
    resetKey.dispose();
  }
}

class EditorScope extends InheritedWidget {
  const EditorScope({required this.editorContext, required super.child, super.key});
  final EditorContext editorContext;

  static EditorContext of(BuildContext context) {
    return context.getInheritedWidgetOfExactType<EditorScope>()!.editorContext;
  }

  @override
  bool updateShouldNotify(covariant EditorScope old) => false;
}
