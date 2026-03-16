import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/controller/input.dart';
import 'package:typie/screens/native_editor/state/scroll_mode.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/view/input.dart';

TextSelection _parseSelection(Map<String, dynamic> json) {
  return TextSelection(baseOffset: json['baseOffset'] as int, extentOffset: json['extentOffset'] as int);
}

TextRange _parseRange(Map<String, dynamic> json) {
  return TextRange(start: json['start'] as int, end: json['end'] as int);
}

TextEditingValue _parseValue(Map<String, dynamic> json) {
  return TextEditingValue(
    text: json['text'] as String,
    selection: _parseSelection(json['selection'] as Map<String, dynamic>),
    composing: _parseRange(json['composing'] as Map<String, dynamic>),
  );
}

TextEditingDelta _deserializeDelta(Map<String, dynamic> json) {
  final oldText = json['oldText'] as String;
  final selection = _parseSelection(json['selection'] as Map<String, dynamic>);
  final composing = _parseRange(json['composing'] as Map<String, dynamic>);

  return switch (json['type']) {
    'insertion' => TextEditingDeltaInsertion(
      oldText: oldText,
      textInserted: json['textInserted'] as String,
      insertionOffset: json['insertionOffset'] as int,
      selection: selection,
      composing: composing,
    ),
    'deletion' => TextEditingDeltaDeletion(
      oldText: oldText,
      deletedRange: _parseRange(json['deletedRange'] as Map<String, dynamic>),
      selection: selection,
      composing: composing,
    ),
    'replacement' => TextEditingDeltaReplacement(
      oldText: oldText,
      replacedRange: _parseRange(json['replacedRange'] as Map<String, dynamic>),
      replacementText: json['replacementText'] as String,
      selection: selection,
      composing: composing,
    ),
    'nonTextUpdate' => TextEditingDeltaNonTextUpdate(oldText: oldText, selection: selection, composing: composing),
    _ => throw ArgumentError('Unknown delta type: ${json['type']}'),
  };
}

const _fixtureDir = 'test/fixtures/input';

List<(String name, File file)> _discoverFixtures() {
  final dir = Directory(_fixtureDir);
  if (!dir.existsSync()) {
    return [];
  }

  final files = dir.listSync().whereType<File>().where((f) => f.path.endsWith('.json')).toList();

  return files.map((f) {
    final basename = f.uri.pathSegments.last;
    final name = basename.substring(0, basename.length - '.json'.length);
    return (name, f);
  }).toList()..sort((a, b) => a.$1.compareTo(b.$1));
}

typedef _Entry = ({
  TextEditingValue? before,
  TextEditingValue? after,
  TextEditingValue? value,
  List<TextEditingDelta> deltas,
  List<Map<String, dynamic>> dispatches,
});

List<_Entry> _loadEntries(File file) {
  final json = jsonDecode(file.readAsStringSync()) as Map<String, dynamic>;
  final entries = json['entries'] as List<dynamic>;
  return entries.map((e) {
    final entry = e as Map<String, dynamic>;
    final type = entry['type'] as String;

    return switch (type) {
      'reconcile' => (
        before: null,
        after: null,
        value: _parseValue(entry['currentValue'] as Map<String, dynamic>),
        deltas: <TextEditingDelta>[],
        dispatches: <Map<String, dynamic>>[],
      ),
      'setEditingState' => (
        before: null,
        after: null,
        value: _parseValue(entry['value'] as Map<String, dynamic>),
        deltas: <TextEditingDelta>[],
        dispatches: <Map<String, dynamic>>[],
      ),
      'batch' => (
        before: _parseValue(entry['before'] as Map<String, dynamic>),
        after: _parseValue(entry['after'] as Map<String, dynamic>),
        value: null,
        deltas: (entry['deltas'] as List<dynamic>).map((d) {
          final map = d as Map<String, dynamic>;
          return _deserializeDelta(map['delta'] as Map<String, dynamic>);
        }).toList(),
        dispatches: (entry['dispatches'] as List<dynamic>?)?.cast<Map<String, dynamic>>() ?? [],
      ),
      _ => throw ArgumentError('Unsupported fixture entry type: $type'),
    };
  }).toList();
}

void main() {
  late List<Map<String, dynamic>> dispatched;
  late GlobalKey<EditorTextInputState> inputKey;
  late InputController controller;

  setUp(() {
    dispatched = [];
    inputKey = GlobalKey<EditorTextInputState>();
    controller = InputController(
      inputKey: inputKey,
      dispatch: dispatched.add,
      editor: NativeEditor.test(),
      onFocusChanged: (_) {},
      scrollIntoView: ({ScrollMode mode = ScrollMode.auto}) {},
      getBottomToolbarMode: () => BottomToolbarMode.hidden,
      getEditorSelection: () => null,
    );
  });

  Future<EditorTextInputState> pumpAndActivate(WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: EditorTextInput(key: inputKey, brightness: Brightness.light, controller: controller),
      ),
    );
    await tester.pump();
    inputKey.currentState!.activateInput();
    return inputKey.currentState!;
  }

  Future<void> sendChord(WidgetTester tester, List<LogicalKeyboardKey> modifiers, LogicalKeyboardKey key) async {
    for (final modifier in modifiers) {
      await tester.sendKeyDownEvent(modifier);
    }
    await tester.sendKeyDownEvent(key);
    await tester.sendKeyUpEvent(key);
    for (final modifier in modifiers.reversed) {
      await tester.sendKeyUpEvent(modifier);
    }
  }

  final fixtures = _discoverFixtures();
  for (final (name, file) in fixtures) {
    testWidgets('fixture: $name', (tester) async {
      final state = await pumpAndActivate(tester);
      final entries = _loadEntries(file);

      for (var i = 0; i < entries.length; i++) {
        final entry = entries[i];

        if (entry.value case final value?) {
          state.syncCurrentValueForTest(value);
          continue;
        }

        state.syncCurrentValueForTest(entry.before!);
        dispatched.clear();
        state.updateEditingValueWithDeltas(entry.deltas);
        expect(dispatched, entry.dispatches, reason: 'entry $i dispatches');
        expect(state.currentTextEditingValue, entry.after, reason: 'entry $i value');
      }
    });
  }

  testWidgets('input controller reconcile does not dispatch commitPreedit for ordinary selection sync', (tester) async {
    EditorSelection? selection;
    controller = InputController(
      inputKey: inputKey,
      dispatch: dispatched.add,
      editor: NativeEditor.test(),
      onFocusChanged: (_) {},
      scrollIntoView: ({ScrollMode mode = ScrollMode.auto}) {},
      getBottomToolbarMode: () => BottomToolbarMode.hidden,
      getEditorSelection: () => selection,
    );

    await pumpAndActivate(tester);

    selection = const EditorSelection(
      range: {
        'anchor': {'nodeId': 'node-1', 'offset': 3, 'affinity': 'upstream'},
        'head': {'nodeId': 'node-1', 'offset': 3, 'affinity': 'upstream'},
      },
      precedingText: 'abc',
      followingText: 'def',
    );
    controller.reconcile();
    expect(dispatched, isEmpty);

    selection = const EditorSelection(
      range: {
        'anchor': {'nodeId': 'node-1', 'offset': 4, 'affinity': 'upstream'},
        'head': {'nodeId': 'node-1', 'offset': 4, 'affinity': 'upstream'},
      },
      precedingText: 'abcd',
      followingText: 'ef',
    );
    controller.reconcile();
    expect(dispatched, isEmpty);
  });

  testWidgets('local input ignores modified backspace shortcuts', (tester) async {
    final state = await pumpAndActivate(tester);

    state.reconcile('node-1', 3, 'abc', 'def');
    dispatched.clear();

    await sendChord(tester, [LogicalKeyboardKey.metaLeft], LogicalKeyboardKey.backspace);
    expect(dispatched, isEmpty);

    await sendChord(tester, [LogicalKeyboardKey.controlLeft], LogicalKeyboardKey.backspace);
    expect(dispatched, isEmpty);

    await sendChord(tester, [LogicalKeyboardKey.altLeft], LogicalKeyboardKey.backspace);
    expect(dispatched, isEmpty);
  });
}
