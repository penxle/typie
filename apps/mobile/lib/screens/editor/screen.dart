import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/editor/editor.dart';
import 'package:typie/screens/editor/schema.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/widgets/webview.dart';

@RoutePage()
class EditorScreen extends HookWidget {
  const EditorScreen({required this.slug, super.key});

  final String slug;

  @override
  Widget build(BuildContext context) {
    final child = Editor(slug: slug);

    final webViewController = useValueNotifier<WebViewController?>(null);
    final proseMirrorState = useValueNotifier<ProseMirrorState?>(null);
    final keyboardHeight = useValueNotifier<double>(0);
    final isKeyboardVisible = useValueNotifier<bool>(false);
    final selectedToolboxIdx = useValueNotifier<int>(-1);
    final selectedTextbarIdx = useValueNotifier<int>(-1);

    return EditorStateScope(
      webViewController: webViewController,
      proseMirrorState: proseMirrorState,
      keyboardHeight: keyboardHeight,
      isKeyboardVisible: isKeyboardVisible,
      selectedToolboxIdx: selectedToolboxIdx,
      selectedTextbarIdx: selectedTextbarIdx,
      child: child,
    );
  }
}
