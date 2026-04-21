import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/controller/upload.dart';
import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/state/theme.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/text.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/text_options/line_height.dart';
import 'package:typie/services/keyboard.dart';
import 'package:typie/styles/semantic_colors.dart';

Future<void> _pumpToolbar(
  WidgetTester tester, {
  required Widget child,
  required void Function(Map<String, dynamic> message) dispatch,
  required VoidCallback requestFocus,
  required VoidCallback reconcileInput,
  ValueNotifier<EditorSelection?>? selection,
  ValueNotifier<List<Map<String, dynamic>>>? attrs,
}) async {
  final controller = EditorController(editor: NativeEditor.test(), fontManager: null);

  await tester.pumpWidget(
    MaterialApp(
      theme: ThemeData(extensions: const <ThemeExtension<dynamic>>[SemanticColors.light]),
      home: NativeEditorToolbarScope(
        controller: controller,
        keyboardHeight: ValueNotifier(0),
        isKeyboardVisible: ValueNotifier(true),
        keyboardType: ValueNotifier(KeyboardType.software),
        isEditorFocused: ValueNotifier(true),
        bottomToolbarMode: ValueNotifier(BottomToolbarMode.hidden),
        secondaryToolbarMode: ValueNotifier(SecondaryToolbarMode.text),
        selection:
            selection ??
            ValueNotifier(
              const EditorSelection(
                range: {
                  'anchor': {'nodeId': 'node-1', 'offset': 3, 'affinity': 'upstream'},
                  'head': {'nodeId': 'node-1', 'offset': 3, 'affinity': 'upstream'},
                },
              ),
            ),
        attrs: attrs ?? ValueNotifier(const []),
        floatingContext: ValueNotifier(null),
        floatingNodeId: ValueNotifier(null),
        externalElements: ValueNotifier(const []),
        uploadManager: UploadManager(),
        dispatch: dispatch,
        requestFocus: requestFocus,
        clearFocus: () {},
        dismissKeyboard: () {},
        reconcileInput: reconcileInput,
        child: Scaffold(body: child),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  setUpAll(() async {
    await initEditorTheme();
  });

  testWidgets('bold toolbar prepares input before toggling style', (tester) async {
    final events = <String>[];

    await _pumpToolbar(
      tester,
      child: const NativeEditorTextToolbar(),
      dispatch: (message) => events.add('dispatch:${message['type']}'),
      requestFocus: () => events.add('requestFocus'),
      reconcileInput: () => events.add('reconcileInput'),
    );

    final gesture = await tester.startGesture(tester.getCenter(find.byIcon(LucideLightIcons.bold)));
    await tester.pump(kPressTimeout);

    expect(events, ['reconcileInput', 'requestFocus']);

    await gesture.up();
    await tester.pumpAndSettle();

    expect(events, ['reconcileInput', 'requestFocus', 'dispatch:toggleBold']);
  });

  testWidgets('line height option prepares input before dispatch', (tester) async {
    final events = <String>[];

    await _pumpToolbar(
      tester,
      child: const NativeEditorLineHeightTextOptionsToolbar(),
      dispatch: (message) => events.add('dispatch:${message['type']}'),
      requestFocus: () => events.add('requestFocus'),
      reconcileInput: () => events.add('reconcileInput'),
      attrs: ValueNotifier(const []),
    );

    final gesture = await tester.startGesture(tester.getCenter(find.text('80%')));
    await tester.pump(kPressTimeout);

    expect(events, ['reconcileInput', 'requestFocus']);

    await gesture.up();
    await tester.pumpAndSettle();

    expect(events, ['reconcileInput', 'requestFocus', 'dispatch:setLineHeight']);
  });
}
