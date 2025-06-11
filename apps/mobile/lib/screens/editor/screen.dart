import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/editor/__generated__/editor_query.data.gql.dart';
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
    final data = useValueNotifier<GEditorScreen_QueryData?>(null);
    final webViewController = useValueNotifier<WebViewController?>(null);
    final proseMirrorState = useValueNotifier<ProseMirrorState?>(null);
    final characterCountState = useValueNotifier<CharacterCountState?>(null);
    final keyboardHeight = useValueNotifier<double>(0);
    final isKeyboardVisible = useValueNotifier<bool>(false);
    final bottomToolbarMode = useValueNotifier<BottomToolbarMode>(BottomToolbarMode.hidden);
    final secondaryToolbarMode = useValueNotifier<SecondaryToolbarMode>(SecondaryToolbarMode.hidden);

    return EditorStateScope(
      data: data,
      webViewController: webViewController,
      proseMirrorState: proseMirrorState,
      characterCountState: characterCountState,
      keyboardHeight: keyboardHeight,
      isKeyboardVisible: isKeyboardVisible,
      bottomToolbarMode: bottomToolbarMode,
      secondaryToolbarMode: secondaryToolbarMode,
      child: Editor(slug: slug),
    );
  }
}
