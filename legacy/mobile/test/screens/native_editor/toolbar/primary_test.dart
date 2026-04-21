import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/controller/upload.dart';
import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/screens/native_editor/toolbar/primary/primary.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/services/keyboard.dart';
import 'package:typie/styles/semantic_colors.dart';

void main() {
  Future<void> pumpToolbar(
    WidgetTester tester, {
    required void Function(Map<String, dynamic> message) dispatch,
    required VoidCallback requestFocus,
    required VoidCallback reconcileInput,
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
          secondaryToolbarMode: ValueNotifier(SecondaryToolbarMode.hidden),
          selection: ValueNotifier(null),
          attrs: ValueNotifier(const []),
          floatingContext: ValueNotifier(null),
          floatingNodeId: ValueNotifier(null),
          externalElements: ValueNotifier(const []),
          uploadManager: UploadManager(),
          dispatch: dispatch,
          requestFocus: requestFocus,
          clearFocus: () {},
          dismissKeyboard: () {},
          reconcileInput: reconcileInput,
          child: const Scaffold(body: NativeEditorPrimaryToolbar()),
        ),
      ),
    );
    await tester.pump();
  }

  testWidgets('redo toolbar reconciles input and keeps focus before dispatch', (tester) async {
    final events = <String>[];

    await pumpToolbar(
      tester,
      dispatch: (message) => events.add('dispatch:${message['type']}'),
      requestFocus: () => events.add('requestFocus'),
      reconcileInput: () => events.add('reconcileInput'),
    );

    final gesture = await tester.startGesture(tester.getCenter(find.byIcon(LucideLightIcons.redo)));
    await tester.pump(kPressTimeout);

    expect(events, ['reconcileInput', 'requestFocus']);

    await gesture.up();
    await tester.pumpAndSettle();

    expect(events, ['reconcileInput', 'requestFocus', 'dispatch:redo']);
  });

  testWidgets('undo toolbar reconciles input and keeps focus before dispatch', (tester) async {
    final events = <String>[];

    await pumpToolbar(
      tester,
      dispatch: (message) => events.add('dispatch:${message['type']}'),
      requestFocus: () => events.add('requestFocus'),
      reconcileInput: () => events.add('reconcileInput'),
    );

    final gesture = await tester.startGesture(tester.getCenter(find.byIcon(LucideLightIcons.undo)));
    await tester.pump(kPressTimeout);

    expect(events, ['reconcileInput', 'requestFocus']);

    await gesture.up();
    await tester.pumpAndSettle();

    expect(events, ['reconcileInput', 'requestFocus', 'dispatch:undo']);
  });
}
