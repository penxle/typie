// Uses legacy synthetic key-data helpers because WidgetTester does not expose an
// equivalent path for the IME-remapped physical-key fallback coverage below.
// ignore_for_file: deprecated_member_use

import 'dart:ui' as ui;

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

Future<void> _sendChord(
  WidgetTester tester,
  List<LogicalKeyboardKey> modifiers,
  LogicalKeyboardKey key, {
  PhysicalKeyboardKey? physicalKey,
  String? character,
}) async {
  for (final modifier in modifiers) {
    await tester.sendKeyDownEvent(modifier);
  }
  await tester.sendKeyDownEvent(key, physicalKey: physicalKey, character: character);
  await tester.sendKeyUpEvent(key, physicalKey: physicalKey);
  for (final modifier in modifiers.reversed) {
    await tester.sendKeyUpEvent(modifier);
  }
}

Future<void> _sendSyntheticKeyEvent(
  WidgetTester tester, {
  required ui.KeyEventType type,
  required LogicalKeyboardKey logicalKey,
  required PhysicalKeyboardKey physicalKey,
  String? character,
}) async {
  ServicesBinding.instance.keyEventManager.handleKeyData(
    ui.KeyData(
      type: type,
      physical: physicalKey.usbHidUsage,
      logical: logicalKey.keyId,
      timeStamp: Duration.zero,
      character: character,
      synthesized: true,
    ),
  );

  await tester.pump();
}

void main() {
  testWidgets('ctrl shortcut works on iOS without relying on meta', (tester) async {
    final shortcuts = <String>[];
    final handler = KeyboardHandler(
      dispatch: (_) {},
      reconcileInput: () {},
      scrollIntoView: ({ScrollMode mode = ScrollMode.auto, bool waitForCursorUpdate = false}) {},
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
      scrollIntoView: ({ScrollMode mode = ScrollMode.auto, bool waitForCursorUpdate = false}) {},
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
      scrollIntoView: ({ScrollMode mode = ScrollMode.auto, bool waitForCursorUpdate = false}) {},
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
      scrollIntoView: ({ScrollMode mode = ScrollMode.auto, bool waitForCursorUpdate = false}) {},
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
      scrollIntoView: ({ScrollMode mode = ScrollMode.auto, bool waitForCursorUpdate = false}) {},
      onShortcut: shortcuts.add,
    );

    await _pumpHarness(tester, handler);
    await _sendChord(tester, [LogicalKeyboardKey.controlLeft], LogicalKeyboardKey.backspace);

    expect(shortcuts, ['deleteWordBackward']);
  }, variant: TargetPlatformVariant.only(TargetPlatform.iOS));

  testWidgets('meta backspace deletes to the start of the line', (tester) async {
    final shortcuts = <String>[];
    final handler = KeyboardHandler(
      dispatch: (_) {},
      reconcileInput: () {},
      scrollIntoView: ({ScrollMode mode = ScrollMode.auto, bool waitForCursorUpdate = false}) {},
      onShortcut: shortcuts.add,
    );

    await _pumpHarness(tester, handler);
    await _sendChord(tester, [LogicalKeyboardKey.metaLeft], LogicalKeyboardKey.backspace);

    expect(shortcuts, ['deleteToLineStart']);
  }, variant: TargetPlatformVariant.only(TargetPlatform.iOS));

  testWidgets('shortcut letters still work when IME reports a non latin logical key', (tester) async {
    final shortcuts = <String>[];
    final handler = KeyboardHandler(
      dispatch: (_) {},
      reconcileInput: () {},
      scrollIntoView: ({ScrollMode mode = ScrollMode.auto, bool waitForCursorUpdate = false}) {},
      onShortcut: shortcuts.add,
    );

    await _pumpHarness(tester, handler);
    await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
    await _sendSyntheticKeyEvent(
      tester,
      type: ui.KeyEventType.down,
      logicalKey: LogicalKeyboardKey('ㅠ'.runes.single),
      physicalKey: PhysicalKeyboardKey.keyB,
      character: 'ㅠ',
    );
    await _sendSyntheticKeyEvent(
      tester,
      type: ui.KeyEventType.up,
      logicalKey: LogicalKeyboardKey('ㅠ'.runes.single),
      physicalKey: PhysicalKeyboardKey.keyB,
    );
    await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);

    expect(shortcuts, ['toggleBold']);
  });

  testWidgets('physical fallback does not trigger for latin layout remaps', (tester) async {
    final shortcuts = <String>[];
    final handler = KeyboardHandler(
      dispatch: (_) {},
      reconcileInput: () {},
      scrollIntoView: ({ScrollMode mode = ScrollMode.auto, bool waitForCursorUpdate = false}) {},
      onShortcut: shortcuts.add,
    );

    await _pumpHarness(tester, handler);
    await _sendChord(
      tester,
      [LogicalKeyboardKey.controlLeft],
      LogicalKeyboardKey.keyQ,
      physicalKey: PhysicalKeyboardKey.keyA,
      character: 'q',
    );

    expect(shortcuts, isEmpty);
  });

  testWidgets('navigation waits for cursor update before consuming scroll intent', (tester) async {
    final scrollCalls = <({ScrollMode mode, bool waitForCursorUpdate})>[];
    final handler = KeyboardHandler(
      dispatch: (_) {},
      reconcileInput: () {},
      scrollIntoView: ({ScrollMode mode = ScrollMode.auto, bool waitForCursorUpdate = false}) {
        scrollCalls.add((mode: mode, waitForCursorUpdate: waitForCursorUpdate));
      },
      onShortcut: (_) {},
    );

    await _pumpHarness(tester, handler);
    await tester.sendKeyDownEvent(LogicalKeyboardKey.arrowUp);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.arrowUp);

    expect(scrollCalls, [(mode: ScrollMode.typewriter, waitForCursorUpdate: true)]);
  });
}
