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

List<TextEditingDelta> _synthesizeKeyEventDeltas(Map<String, dynamic> entry) {
  final before = entry['before'] as Map<String, dynamic>;
  final after = entry['after'] as Map<String, dynamic>;
  final oldText = before['text'] as String;
  final newText = after['text'] as String;
  final selection = _parseSelection(after['selection'] as Map<String, dynamic>);
  final composing = _parseRange(after['composing'] as Map<String, dynamic>);

  if (newText.length < oldText.length) {
    final beforeSel = before['selection'] as Map<String, dynamic>;
    final deletedEnd = beforeSel['baseOffset'] as int;
    final deletedStart = deletedEnd - (oldText.length - newText.length);
    return [
      TextEditingDeltaDeletion(
        oldText: oldText,
        deletedRange: TextRange(start: deletedStart, end: deletedEnd),
        selection: selection,
        composing: composing,
      ),
    ];
  }

  return [TextEditingDeltaNonTextUpdate(oldText: oldText, selection: selection, composing: composing)];
}

typedef _Entry = ({TextEditingValue before, List<TextEditingDelta> deltas, List<Map<String, dynamic>> dispatches});

List<_Entry> _loadEntries(File file) {
  final json = jsonDecode(file.readAsStringSync()) as Map<String, dynamic>;
  final entries = json['entries'] as List<dynamic>;
  return entries.map((e) {
    final entry = e as Map<String, dynamic>;
    final before = _parseValue(entry['before'] as Map<String, dynamic>);
    final dispatches = (entry['dispatches'] as List<dynamic>?)?.cast<Map<String, dynamic>>() ?? [];

    if (entry['type'] == 'keyEvent') {
      return (before: before, deltas: _synthesizeKeyEventDeltas(entry), dispatches: dispatches);
    }

    final deltas = (entry['deltas'] as List<dynamic>).map((d) {
      final map = d as Map<String, dynamic>;
      return _deserializeDelta(map['delta'] as Map<String, dynamic>);
    }).toList();

    return (before: before, deltas: deltas, dispatches: dispatches);
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

  final fixtures = _discoverFixtures();
  for (final (name, file) in fixtures) {
    testWidgets('fixture: $name', (tester) async {
      final state = await pumpAndActivate(tester);
      final entries = _loadEntries(file);

      if (entries.isNotEmpty) {
        final firstBefore = entries.first.before;
        final selection = firstBefore.selection;
        final text = firstBefore.text;
        final splitOffsetRaw = selection.isValid && selection.isCollapsed ? selection.baseOffset : text.length;
        final splitOffset = splitOffsetRaw < 0
            ? 0
            : splitOffsetRaw > text.length
            ? text.length
            : splitOffsetRaw;
        state.reconcile('fixture:$name', splitOffset, text.substring(0, splitOffset), text.substring(splitOffset));
      }

      for (var i = 0; i < entries.length; i++) {
        dispatched.clear();
        state.updateEditingValueWithDeltas(entries[i].deltas);
        expect(dispatched, entries[i].dispatches, reason: 'entry $i');
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
}
