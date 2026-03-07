import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:typie/screens/native_editor/controller/keyboard.dart';
import 'package:typie/screens/native_editor/state/scroll_mode.dart';

class _KeyboardHarness extends StatelessWidget {
  const _KeyboardHarness({required this.handler});

  final KeyboardHandler handler;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      home: Focus(
        autofocus: true,
        onKeyEvent: (_, event) => handler.handleKeyEvent(event) ? KeyEventResult.handled : KeyEventResult.ignored,
        child: const SizedBox.expand(),
      ),
    );
  }
}

Future<void> _pumpHarness(WidgetTester tester, KeyboardHandler handler) async {
  await tester.pumpWidget(_KeyboardHarness(handler: handler));
  await tester.pump();
}

Future<void> _sendChord(WidgetTester tester, List<LogicalKeyboardKey> modifiers, LogicalKeyboardKey key) async {
  for (final modifier in modifiers) {
    await tester.sendKeyDownEvent(modifier);
  }
  await tester.sendKeyDownEvent(key);
  await tester.sendKeyUpEvent(key);
  for (final modifier in modifiers.reversed) {
    await tester.sendKeyUpEvent(modifier);
  }
}

void main() {
  testWidgets('ctrl shortcut works on iOS without relying on meta', (tester) async {
    final shortcuts = <String>[];
    final handler = KeyboardHandler(
      dispatch: (_) {},
      reconcileInput: () {},
      scrollIntoView: ({ScrollMode mode = ScrollMode.auto}) {},
      onShortcut: shortcuts.add,
    );

    await _pumpHarness(tester, handler);
    await _sendChord(tester, [LogicalKeyboardKey.controlLeft], LogicalKeyboardKey.keyB);

    expect(shortcuts, ['toggleBold']);
  }, variant: TargetPlatformVariant.only(TargetPlatform.iOS));

  testWidgets('meta shortcut works on Android without relying on ctrl', (tester) async {
    final shortcuts = <String>[];
    final handler = KeyboardHandler(
      dispatch: (_) {},
      reconcileInput: () {},
      scrollIntoView: ({ScrollMode mode = ScrollMode.auto}) {},
      onShortcut: shortcuts.add,
    );

    await _pumpHarness(tester, handler);
    await _sendChord(tester, [LogicalKeyboardKey.metaLeft], LogicalKeyboardKey.keyB);

    expect(shortcuts, ['toggleBold']);
  }, variant: TargetPlatformVariant.only(TargetPlatform.android));

  testWidgets('ctrl arrow navigation still moves by word on iOS', (tester) async {
    final dispatches = <Map<String, dynamic>>[];
    final handler = KeyboardHandler(
      dispatch: dispatches.add,
      reconcileInput: () {},
      scrollIntoView: ({ScrollMode mode = ScrollMode.auto}) {},
      onShortcut: (_) {},
    );

    await _pumpHarness(tester, handler);
    await _sendChord(tester, [LogicalKeyboardKey.controlLeft], LogicalKeyboardKey.arrowLeft);

    expect(dispatches, [
      {'type': 'navigate', 'direction': 'wordLeft', 'extend': false},
    ]);
  }, variant: TargetPlatformVariant.only(TargetPlatform.iOS));

  testWidgets('alt arrow navigation moves by word on Android', (tester) async {
    final dispatches = <Map<String, dynamic>>[];
    final handler = KeyboardHandler(
      dispatch: dispatches.add,
      reconcileInput: () {},
      scrollIntoView: ({ScrollMode mode = ScrollMode.auto}) {},
      onShortcut: (_) {},
    );

    await _pumpHarness(tester, handler);
    await _sendChord(tester, [LogicalKeyboardKey.altLeft], LogicalKeyboardKey.arrowLeft);

    expect(dispatches, [
      {'type': 'navigate', 'direction': 'wordLeft', 'extend': false},
    ]);
  }, variant: TargetPlatformVariant.only(TargetPlatform.android));

  testWidgets('ctrl backspace deletes a word when meta is unavailable', (tester) async {
    final shortcuts = <String>[];
    final handler = KeyboardHandler(
      dispatch: (_) {},
      reconcileInput: () {},
      scrollIntoView: ({ScrollMode mode = ScrollMode.auto}) {},
      onShortcut: shortcuts.add,
    );

    await _pumpHarness(tester, handler);
    await _sendChord(tester, [LogicalKeyboardKey.controlLeft], LogicalKeyboardKey.backspace);

    expect(shortcuts, ['deleteWordBackward']);
  }, variant: TargetPlatformVariant.only(TargetPlatform.iOS));
}
