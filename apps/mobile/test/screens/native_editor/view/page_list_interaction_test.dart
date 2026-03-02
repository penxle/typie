import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/controller/dnd_controller.dart';
import 'package:typie/screens/native_editor/controller/input.dart';
import 'package:typie/screens/native_editor/controller/ticker.dart';
import 'package:typie/screens/native_editor/controller/upload.dart';
import 'package:typie/screens/native_editor/external/models.dart';
import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/view/context_menu.dart';
import 'package:typie/screens/native_editor/view/input.dart';
import 'package:typie/screens/native_editor/view/interaction/controller.dart';
import 'package:typie/screens/native_editor/view/interaction/mode.dart';
import 'package:typie/screens/native_editor/view/interaction/state.dart';
import 'package:typie/screens/native_editor/view/pages.dart';
import 'package:typie/screens/native_editor/view/scope.dart';
import 'package:typie/screens/native_editor/view/selection.dart';
import 'package:typie/service.dart';
import 'package:typie/services/keyboard.dart';
import 'package:typie/services/preference.dart';
import 'package:typie/styles/theme_data.dart';

class _TestPref implements Pref {
  @override
  String siteId = 'test-site';

  @override
  bool devMode = false;

  @override
  bool typewriterEnabled = false;

  @override
  double typewriterPosition = 0.5;

  @override
  bool lineHighlightEnabled = false;

  @override
  String pasteMode = 'ask';

  @override
  bool autoSurroundEnabled = true;

  @override
  Map<String, double>? characterCountFloatingPosition;

  @override
  bool characterCountFloatingEnabled = false;

  @override
  bool widgetAutoFadeEnabled = true;
}

BottomToolbarMode _hiddenBottomToolbarMode() => BottomToolbarMode.hidden;

class _ImmediateEditorTicker extends EditorTicker {
  _ImmediateEditorTicker({required super.getController}) : super(tickerProvider: const TestVSync());

  @override
  Future<void> settled() {
    return Future.value();
  }

  @override
  void start() {}

  @override
  void stop() {}
}

class _PageListHarnessDeps {
  _PageListHarnessDeps._({
    required this.editor,
    required this.controller,
    required this.ticker,
    required this.dndController,
    required this.inputController,
    required this.interactionState,
    required this.verticalScrollController,
    required this.horizontalScrollController,
    required this.longPressPosition,
    required this.handleDragPosition,
    required this.titleAreaHeight,
    required this.title,
    required this.subtitle,
    required this.pendingScroll,
    required this.pendingScrollPageIdx,
    required this.presentedViewport,
    required this.displayZoom,
    required this.renderZoom,
    required this.keyboardHeight,
    required this.isKeyboardVisible,
    required this.keyboardType,
    required this.isEditorFocused,
    required this.bottomToolbarMode,
    required this.secondaryToolbarMode,
    required this.selection,
    required this.attrs,
    required this.externalElements,
    required this.uploadManager,
    required this.titleFocusNode,
    required this.subtitleFocusNode,
  });

  factory _PageListHarnessDeps.create({
    bool selectionHit = false,
    bool interactiveHit = false,
    Map<String, dynamic>? clipboardData,
    bool immediateSettledTicker = false,
  }) {
    final editor = NativeEditor.test(
      selectionHit: selectionHit,
      interactiveHit: interactiveHit,
      clipboardData: clipboardData,
    );
    final controller = EditorController(editor: editor, fontManager: null)
      ..updateState(
        (_) => const EditorState(
          layout: Layout.paginated(
            pageWidth: 840,
            pageHeight: 2200,
            pageMarginTop: 72,
            pageMarginBottom: 72,
            pageMarginLeft: 72,
            pageMarginRight: 72,
          ),
          pages: [PageSize(width: 840, height: 2200)],
          isFocused: true,
        ),
      );

    final verticalScrollController = ScrollController();
    final horizontalScrollController = ScrollController();
    final interactionState = EditorInteractionState();
    final titleAreaHeight = ValueNotifier<double>(0);
    final displayZoom = ValueNotifier<double>(1);
    final renderZoom = ValueNotifier<double>(1);

    final inputController = InputController(
      inputKey: GlobalKey<InputViewState>(),
      dispatch: controller.dispatch,
      editor: editor,
      onFocusChanged: controller.setFocused,
      scrollIntoView: controller.scrollIntoView,
      getBottomToolbarMode: _hiddenBottomToolbarMode,
    );

    EditorController readController() => controller;

    final dndController = DndController(editor: editor, controller: controller);
    final ticker = immediateSettledTicker
        ? _ImmediateEditorTicker(getController: readController)
        : EditorTicker(getController: readController, tickerProvider: const TestVSync());

    return _PageListHarnessDeps._(
      editor: editor,
      controller: controller,
      ticker: ticker,
      dndController: dndController,
      inputController: inputController,
      interactionState: interactionState,
      verticalScrollController: verticalScrollController,
      horizontalScrollController: horizontalScrollController,
      longPressPosition: ValueNotifier<Offset?>(null),
      handleDragPosition: ValueNotifier<Offset?>(null),
      titleAreaHeight: titleAreaHeight,
      title: ValueNotifier<String>(''),
      subtitle: ValueNotifier<String>(''),
      pendingScroll: ValueNotifier<VoidCallback?>(null),
      pendingScrollPageIdx: ValueNotifier<int?>(null),
      presentedViewport: ValueNotifier(const PresentedViewport.base(cursor: null, renderVersion: null)),
      displayZoom: displayZoom,
      renderZoom: renderZoom,
      keyboardHeight: ValueNotifier<double>(0),
      isKeyboardVisible: ValueNotifier<bool>(false),
      keyboardType: ValueNotifier<KeyboardType>(KeyboardType.software),
      isEditorFocused: ValueNotifier<bool>(true),
      bottomToolbarMode: ValueNotifier<BottomToolbarMode>(BottomToolbarMode.hidden),
      secondaryToolbarMode: ValueNotifier<SecondaryToolbarMode>(SecondaryToolbarMode.hidden),
      selection: ValueNotifier<EditorSelection?>(null),
      attrs: ValueNotifier<List<Map<String, dynamic>>>(const []),
      externalElements: ValueNotifier(const []),
      uploadManager: UploadManager(),
      titleFocusNode: FocusNode(),
      subtitleFocusNode: FocusNode(),
    );
  }

  final NativeEditor editor;
  final EditorController controller;
  final EditorTicker ticker;
  final DndController dndController;
  final InputController inputController;
  final EditorInteractionState interactionState;

  final ScrollController verticalScrollController;
  final ScrollController horizontalScrollController;
  final ValueNotifier<Offset?> longPressPosition;
  final ValueNotifier<Offset?> handleDragPosition;
  final ValueNotifier<double> titleAreaHeight;
  final ValueNotifier<String> title;
  final ValueNotifier<String> subtitle;
  final ValueNotifier<VoidCallback?> pendingScroll;
  final ValueNotifier<int?> pendingScrollPageIdx;
  final ValueNotifier<PresentedViewport> presentedViewport;
  final ValueNotifier<double> displayZoom;
  final ValueNotifier<double> renderZoom;
  final ValueNotifier<double> keyboardHeight;
  final ValueNotifier<bool> isKeyboardVisible;
  final ValueNotifier<KeyboardType> keyboardType;
  final ValueNotifier<bool> isEditorFocused;
  final ValueNotifier<BottomToolbarMode> bottomToolbarMode;
  final ValueNotifier<SecondaryToolbarMode> secondaryToolbarMode;
  final ValueNotifier<EditorSelection?> selection;
  final ValueNotifier<List<Map<String, dynamic>>> attrs;
  final ValueNotifier<List<ExternalElement>> externalElements;
  final UploadManager uploadManager;
  final FocusNode titleFocusNode;
  final FocusNode subtitleFocusNode;

  Widget build({bool rebuildContentScopeOnControllerChange = false}) {
    Widget buildContentScope() {
      return ContentScope(
        controller: controller,
        ticker: ticker,
        verticalScrollController: verticalScrollController,
        horizontalScrollController: horizontalScrollController,
        inputController: inputController,
        longPressPosition: longPressPosition,
        handleDragPosition: handleDragPosition,
        titleAreaHeight: titleAreaHeight,
        title: title,
        subtitle: subtitle,
        onTitleChanged: (_) {},
        onSubtitleChanged: (_) {},
        titleFocusNode: titleFocusNode,
        subtitleFocusNode: subtitleFocusNode,
        pendingScroll: pendingScroll,
        pendingScrollPageIdx: pendingScrollPageIdx,
        presentedViewport: presentedViewport,
        dndController: dndController,
        interactionState: interactionState,
        interactionSnapshot: interactionState.listenable,
        displayZoom: displayZoom,
        renderZoom: renderZoom,
        setZoom: (zoom, {bool commitRender = false}) {
          displayZoom.value = zoom;
          renderZoom.value = zoom;
        },
        child: const PageList(),
      );
    }

    final content = rebuildContentScopeOnControllerChange
        ? AnimatedBuilder(animation: controller, builder: (_, __) => buildContentScope())
        : buildContentScope();

    return MaterialApp(
      theme: lightTheme,
      home: Scaffold(
        body: NativeEditorToolbarScope(
          controller: controller,
          keyboardHeight: keyboardHeight,
          isKeyboardVisible: isKeyboardVisible,
          keyboardType: keyboardType,
          isEditorFocused: isEditorFocused,
          bottomToolbarMode: bottomToolbarMode,
          secondaryToolbarMode: secondaryToolbarMode,
          selection: selection,
          attrs: attrs,
          floatingContext: controller.floatingContext,
          floatingNodeId: controller.floatingNodeId,
          externalElements: externalElements,
          uploadManager: uploadManager,
          dispatch: controller.dispatch,
          requestFocus: inputController.requestFocus,
          clearFocus: inputController.clearFocus,
          dismissKeyboard: inputController.dismissKeyboard,
          commitComposing: inputController.commitComposing,
          child: content,
        ),
      ),
    );
  }

  Future<void> dispose() async {
    interactionState.dispose();
    dndController.dispose();
    uploadManager.dispose();
    verticalScrollController.dispose();
    horizontalScrollController.dispose();
    longPressPosition.dispose();
    handleDragPosition.dispose();
    titleAreaHeight.dispose();
    title.dispose();
    subtitle.dispose();
    pendingScroll.dispose();
    pendingScrollPageIdx.dispose();
    presentedViewport.dispose();
    displayZoom.dispose();
    renderZoom.dispose();
    keyboardHeight.dispose();
    isKeyboardVisible.dispose();
    keyboardType.dispose();
    isEditorFocused.dispose();
    bottomToolbarMode.dispose();
    secondaryToolbarMode.dispose();
    selection.dispose();
    attrs.dispose();
    externalElements.dispose();
    titleFocusNode.dispose();
    subtitleFocusNode.dispose();
    controller.dispose();
    if (!editor.isTest) {
      editor.dispose();
    }
  }
}

void main() {
  Future<double> dragPageList(WidgetTester tester, _PageListHarnessDeps deps, {double dy = -220}) async {
    await tester.drag(find.byType(PageList), Offset(0, dy));
    await tester.pump();
    return deps.verticalScrollController.offset;
  }

  EditorSelection seededRangeSelectionWithoutAnchorBounds() {
    return const EditorSelection(
      collapsed: false,
      cmp: 1,
      headBounds: SelectionEndpointBounds(pageIdx: 0, x: 180, y: 320, width: 1, height: 20),
      range: {
        'anchor': {'nodeId': 'n1', 'offset': 3, 'affinity': 'forward'},
        'head': {'nodeId': 'n1', 'offset': 8, 'affinity': 'forward'},
      },
    );
  }

  EditorSelection seededRangeSelection() {
    return const EditorSelection(
      collapsed: false,
      cmp: 1,
      anchorBounds: SelectionEndpointBounds(pageIdx: 0, x: 120, y: 320, width: 1, height: 20),
      headBounds: SelectionEndpointBounds(pageIdx: 0, x: 180, y: 320, width: 1, height: 20),
      range: {
        'anchor': {'nodeId': 'n1', 'offset': 3, 'affinity': 'forward'},
        'head': {'nodeId': 'n1', 'offset': 8, 'affinity': 'forward'},
      },
    );
  }

  CursorInfo seededCursor() {
    return const CursorInfo(pageIdx: 0, x: 160, y: 360, height: 20, visible: true, precedingCharWidths: []);
  }

  Future<void> quickTap(WidgetTester tester, Offset position) async {
    final gesture = await tester.startGesture(position);
    await tester.pump(const Duration(milliseconds: 10));
    await gesture.up();
    await tester.pump(const Duration(milliseconds: 10));
  }

  Offset interactionPoint(WidgetTester tester, _PageListHarnessDeps deps) {
    final target = find.byType(PageList);
    final topLeft = tester.getTopLeft(target);
    final size = tester.getSize(target);
    final localX = (size.width / 2).clamp(40.0, size.width - 40.0);
    final localY = (deps.titleAreaHeight.value + 120).clamp(40.0, size.height - 40.0);
    return topLeft + Offset(localX, localY);
  }

  EditorInteractionController interactionControllerOf(WidgetTester tester) {
    return tester.widget<EditorInteractionControllerScope>(find.byType(EditorInteractionControllerScope)).controller;
  }

  setUp(() async {
    if (serviceLocator.isRegistered<Pref>()) {
      await serviceLocator.unregister<Pref>();
    }
    serviceLocator.registerSingleton<Pref>(_TestPref());
  });

  tearDown(() async {
    if (serviceLocator.isRegistered<Pref>()) {
      await serviceLocator.reset();
    }
  });

  group('PageList interaction regression', () {
    testWidgets('interaction controller instance stays stable across content-scope rebuilds', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build(rebuildContentScopeOnControllerChange: true));
      await tester.pumpAndSettle();

      final before = interactionControllerOf(tester);
      deps.controller.updateState((state) => state.copyWith(selection: seededRangeSelection()));
      await tester.pump();
      final afterSelection = interactionControllerOf(tester);

      deps.controller.updateState((state) => state.copyWith(selection: null));
      await tester.pump();
      final afterClear = interactionControllerOf(tester);

      expect(identical(before, afterSelection), isTrue);
      expect(identical(before, afterClear), isTrue);
    });

    testWidgets('double-tap hold-drag extends selection repeatedly', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final point = interactionPoint(tester, deps);

      await quickTap(tester, point);
      deps.editor.clearTestDispatchedMessages();

      final drag = await tester.startGesture(point);
      await tester.pump(const Duration(milliseconds: 120));
      deps.controller.updateState((state) => state.copyWith(selection: seededRangeSelection()));
      await tester.pump();
      await drag.moveBy(const Offset(0, -80));
      await tester.pump();
      await drag.moveBy(const Offset(0, -80));
      await tester.pump();
      await drag.up();
      await tester.pump();

      final hasDoubleTapDispatch = deps.editor.testDispatchedMessages.any(
        (message) => message['type'] == 'pointerDown' && message['clickCount'] == 2,
      );
      final extendEvents = deps.editor.testDispatchedMessages
          .where((message) => message['type'] == 'extendSelectionTo')
          .toList();

      expect(hasDoubleTapDispatch, isTrue);
      expect(extendEvents.length, greaterThanOrEqualTo(2));
    });

    testWidgets('double-tap hold-drag still extends after content-scope rebuild', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build(rebuildContentScopeOnControllerChange: true));
      await tester.pumpAndSettle();

      final point = interactionPoint(tester, deps);

      await quickTap(tester, point);
      deps.editor.clearTestDispatchedMessages();

      final beforePan = deps.verticalScrollController.offset;
      final drag = await tester.startGesture(point);
      await tester.pump(const Duration(milliseconds: 120));
      deps.controller.updateState((state) => state.copyWith(selection: seededRangeSelection()));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 550));
      await drag.moveBy(const Offset(0, -80));
      await tester.pump();
      await drag.moveBy(const Offset(0, -80));
      await tester.pump();
      await drag.up();
      await tester.pump();
      final afterPan = deps.verticalScrollController.offset;

      final extendEvents = deps.editor.testDispatchedMessages
          .where((message) => message['type'] == 'extendSelectionTo')
          .toList();
      expect(extendEvents.length, greaterThanOrEqualTo(2));
      expect((afterPan - beforePan).abs(), lessThan(0.1));
    });

    testWidgets('double-tap quick drag extends selection and does not pan-scroll', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final point = interactionPoint(tester, deps) + const Offset(0, 180);

      await quickTap(tester, point);
      deps.editor.clearTestDispatchedMessages();

      final beforePan = deps.verticalScrollController.offset;
      final drag = await tester.startGesture(point);
      await tester.pump(const Duration(milliseconds: 120));
      deps.controller.updateState((state) => state.copyWith(selection: seededRangeSelection()));
      await tester.pump();
      await drag.moveBy(const Offset(0, -60));
      await tester.pump();
      await drag.moveBy(const Offset(0, -60));
      await tester.pump();
      await drag.up();
      await tester.pump();
      final afterPan = deps.verticalScrollController.offset;

      final extendEvents = deps.editor.testDispatchedMessages
          .where((message) => message['type'] == 'extendSelectionTo')
          .toList();
      expect(extendEvents.isNotEmpty, isTrue);
      expect((afterPan - beforePan).abs(), lessThan(0.1));
    });

    testWidgets('pan works after double-tap selection', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final point = interactionPoint(tester, deps);

      await quickTap(tester, point);
      await quickTap(tester, point);
      await tester.pump();

      final beforePan = deps.verticalScrollController.offset;
      final pan = await tester.startGesture(point + const Offset(80, 80));
      await tester.pump();
      await pan.moveBy(const Offset(0, -260));
      await tester.pump();
      await pan.up();
      await tester.pump();
      final afterPan = deps.verticalScrollController.offset;

      expect(afterPan, greaterThan(beforePan + 1));
    });

    testWidgets('pan works after double-tap drag completes', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final point = interactionPoint(tester, deps) + const Offset(0, 140);
      await quickTap(tester, point);

      final drag = await tester.startGesture(point);
      await tester.pump(const Duration(milliseconds: 20));
      await drag.moveBy(const Offset(0, -80));
      await tester.pump();
      await drag.up();
      await tester.pump();
      expect(deps.interactionState.snapshot().mode, InteractionMode.idle);

      final beforePan = deps.verticalScrollController.offset;
      final pan = await tester.startGesture(point + const Offset(80, 80));
      await tester.pump();
      await pan.moveBy(const Offset(0, -260));
      await tester.pump();
      await pan.up();
      await tester.pump();
      final afterPan = deps.verticalScrollController.offset;
      expect(afterPan, greaterThan(beforePan + 1));
    });

    testWidgets('double-tap drag auto-scrolls at viewport edge', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final target = find.byType(PageList);
      final topLeft = tester.getTopLeft(target);
      final size = tester.getSize(target);
      final start = topLeft + Offset(size.width / 2, size.height - 12);
      await quickTap(tester, start);
      deps.editor.clearTestDispatchedMessages();

      final before = deps.verticalScrollController.offset;
      final drag = await tester.startGesture(start);
      await tester.pump(const Duration(milliseconds: 120));
      deps.controller.updateState((state) => state.copyWith(selection: seededRangeSelection()));
      await tester.pump();
      await drag.moveBy(const Offset(0, 10));
      await tester.pump(const Duration(milliseconds: 250));
      await drag.up();
      await tester.pump();
      final after = deps.verticalScrollController.offset;

      final extendEvents = deps.editor.testDispatchedMessages
          .where((message) => message['type'] == 'extendSelectionTo')
          .toList();
      expect(extendEvents.isNotEmpty, isTrue);
      expect(after, greaterThan(before));
    });

    testWidgets('double-tap does not emit trailing single click', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final point = interactionPoint(tester, deps);

      await quickTap(tester, point);
      deps.editor.clearTestDispatchedMessages();
      await quickTap(tester, point);
      await tester.pump();

      final pointerDowns = deps.editor.testDispatchedMessages.where((message) => message['type'] == 'pointerDown');
      final click2 = pointerDowns.where((message) => message['clickCount'] == 2).length;
      final click1 = pointerDowns.where((message) => message['clickCount'] == 1).length;
      expect(click2, 1);
      expect(click1, 0);
    });

    testWidgets('double-tap drag does not extend until selection bounds are materialized', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final point = interactionPoint(tester, deps);

      await quickTap(tester, point);
      deps.controller.updateState((state) => state.copyWith(selection: seededRangeSelectionWithoutAnchorBounds()));
      await tester.pump();

      deps.editor.clearTestDispatchedMessages();
      final drag = await tester.startGesture(point);
      await tester.pump(const Duration(milliseconds: 120));
      await drag.moveBy(const Offset(0, -90));
      await tester.pump();
      await drag.up();
      await tester.pump();

      final extendEvent = deps.editor.testDispatchedMessages.firstWhere(
        (message) => message['type'] == 'extendSelectionTo',
        orElse: () => <String, dynamic>{},
      );
      expect(extendEvent.isEmpty, isTrue);
    });

    testWidgets('double-tap selection then handle drag extends selection', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final point = interactionPoint(tester, deps);
      await quickTap(tester, point);
      await quickTap(tester, point);
      deps.controller.updateState((state) => state.copyWith(selection: seededRangeSelection()));
      await tester.pump();

      deps.editor.clearTestDispatchedMessages();
      final handles = find.byType(SelectionHandle);
      expect(handles, findsWidgets);
      await tester.drag(handles.last, const Offset(0, -80), warnIfMissed: false);
      await tester.pump();

      final extendEvent = deps.editor.testDispatchedMessages.firstWhere(
        (message) => message['type'] == 'extendSelectionTo',
        orElse: () => <String, dynamic>{},
      );
      expect(extendEvent.isNotEmpty, isTrue);
    });

    testWidgets('long-press move keeps updating cursor and clears magnifier on release', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final start = interactionPoint(tester, deps);

      deps.editor.clearTestDispatchedMessages();
      final gesture = await tester.startGesture(start);
      await tester.pump(const Duration(milliseconds: 600));

      await gesture.moveBy(const Offset(16, -6));
      await tester.pump();
      await gesture.moveBy(const Offset(18, -8));
      await tester.pump();

      final movePointerDownCount = deps.editor.testDispatchedMessages
          .where((message) => message['type'] == 'pointerDown' && message['clickCount'] == 1)
          .length;
      expect(movePointerDownCount, greaterThanOrEqualTo(2));
      expect(deps.longPressPosition.value, isNotNull);

      await gesture.up();
      await tester.pump();
      expect(deps.longPressPosition.value, isNull);
      expect(deps.interactionState.snapshot().mode, InteractionMode.idle);
    });

    testWidgets('long-press drag keeps dispatching cursor moves after first update', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final start = interactionPoint(tester, deps) + const Offset(0, 120);

      deps.editor.clearTestDispatchedMessages();
      final gesture = await tester.startGesture(start);
      await tester.pump(const Duration(milliseconds: 600));

      await gesture.moveBy(const Offset(0, -20));
      await tester.pump();
      await gesture.moveBy(const Offset(0, -20));
      await tester.pump();
      await gesture.moveBy(const Offset(0, -20));
      await tester.pump();

      final pointerDownEvents = deps.editor.testDispatchedMessages
          .where((message) => message['type'] == 'pointerDown' && message['clickCount'] == 1)
          .toList();
      final uniqueY = pointerDownEvents.map((message) => message['y']).toSet();
      expect(pointerDownEvents.length, greaterThanOrEqualTo(3));
      expect(uniqueY.length, greaterThanOrEqualTo(2));

      await gesture.up();
      await tester.pump();
      expect(deps.longPressPosition.value, isNull);
    });

    testWidgets('long-press drag keeps dispatching after content-scope rebuild', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build(rebuildContentScopeOnControllerChange: true));
      await tester.pumpAndSettle();

      final start = interactionPoint(tester, deps) + const Offset(0, 120);

      deps.editor.clearTestDispatchedMessages();
      final gesture = await tester.startGesture(start);
      await tester.pump(const Duration(milliseconds: 600));

      await gesture.moveBy(const Offset(0, -20));
      await tester.pump();
      await gesture.moveBy(const Offset(0, -20));
      await tester.pump();
      await gesture.moveBy(const Offset(0, -20));
      await tester.pump();

      final pointerDownEvents = deps.editor.testDispatchedMessages
          .where((message) => message['type'] == 'pointerDown' && message['clickCount'] == 1)
          .toList();
      final uniqueY = pointerDownEvents.map((message) => message['y']).toSet();
      expect(pointerDownEvents.length, greaterThanOrEqualTo(3));
      expect(uniqueY.length, greaterThanOrEqualTo(2));

      await gesture.up();
      await tester.pump();
      expect(deps.longPressPosition.value, isNull);
    });

    testWidgets('pan works after long-press cursor move ends', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final start = interactionPoint(tester, deps) + const Offset(0, 120);
      final longPress = await tester.startGesture(start);
      await tester.pump(const Duration(milliseconds: 600));
      await longPress.moveBy(const Offset(0, -30));
      await tester.pump();
      await longPress.up();
      await tester.pump();

      final beforePan = deps.verticalScrollController.offset;
      final pan = await tester.startGesture(start + const Offset(80, 80));
      await tester.pump();
      await pan.moveBy(const Offset(0, -240));
      await tester.pump();
      await pan.up();
      await tester.pump();
      final afterPan = deps.verticalScrollController.offset;
      expect(afterPan, greaterThan(beforePan + 1));
    });

    testWidgets('long-press outside selection moves cursor', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final point = interactionPoint(tester, deps);
      deps.editor.clearTestDispatchedMessages();
      final longPress = await tester.startGesture(point);
      await tester.pump(const Duration(milliseconds: 600));
      await longPress.moveBy(const Offset(10, -10));
      await tester.pump();
      await longPress.up();
      await tester.pump();

      final pointerDowns = deps.editor.testDispatchedMessages
          .where((message) => message['type'] == 'pointerDown' && message['clickCount'] == 1)
          .toList();
      expect(pointerDowns.isNotEmpty, isTrue);
    });

    testWidgets('same-cursor tap toggles context menu open-close-open', (tester) async {
      final deps = _PageListHarnessDeps.create(immediateSettledTicker: true);
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      deps.controller.updateState((state) => state.copyWith(cursor: seededCursor(), selection: null));
      await tester.pump();

      final point = interactionPoint(tester, deps);

      await quickTap(tester, point);
      await tester.pump(const Duration(milliseconds: 200));
      expect(find.byType(SelectionContextMenu), findsOneWidget);

      await tester.pump(const Duration(milliseconds: 350));
      await quickTap(tester, point);
      await tester.pump(const Duration(milliseconds: 200));
      expect(find.byType(SelectionContextMenu), findsNothing);

      await tester.pump(const Duration(milliseconds: 350));
      await quickTap(tester, point);
      await tester.pump(const Duration(milliseconds: 200));
      expect(find.byType(SelectionContextMenu), findsOneWidget);
    });

    testWidgets('double-tap hold keeps double-tap selecting mode even outside selection', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final point = interactionPoint(tester, deps);
      await quickTap(tester, point);

      final hold = await tester.startGesture(point);
      await tester.pump(const Duration(milliseconds: 120));
      deps.controller.updateState((state) => state.copyWith(selection: seededRangeSelection()));
      await tester.pump();
      await hold.moveBy(const Offset(220, 0));
      await tester.pump();
      expect(deps.interactionState.snapshot().mode, InteractionMode.doubleTapSelecting);

      await hold.moveBy(const Offset(0, -60));
      await tester.pump();
      expect(deps.interactionState.snapshot().mode, InteractionMode.doubleTapSelecting);

      await hold.up();
      await tester.pump();
      expect(deps.interactionState.snapshot().mode, InteractionMode.idle);
    });

    testWidgets('double-tap near line end prefers word selection over context menu toggle', (tester) async {
      final deps = _PageListHarnessDeps.create(immediateSettledTicker: true);
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      deps.controller.updateState((state) => state.copyWith(cursor: seededCursor(), selection: null));
      await tester.pump();

      final point = interactionPoint(tester, deps);

      await quickTap(tester, point);
      await tester.pump(const Duration(milliseconds: 200));
      expect(find.byType(SelectionContextMenu), findsOneWidget);

      deps.editor.clearTestDispatchedMessages();
      final farRight = point + const Offset(18, 0);
      await quickTap(tester, farRight);
      await tester.pump(const Duration(milliseconds: 200));

      final pointerDowns = deps.editor.testDispatchedMessages.where((message) => message['type'] == 'pointerDown');
      final click2 = pointerDowns.where((message) => message['clickCount'] == 2).length;
      expect(click2, 1);
    });

    testWidgets('double-tap hold without drag shows context menu on release', (tester) async {
      final deps = _PageListHarnessDeps.create(immediateSettledTicker: true);
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final point = interactionPoint(tester, deps);

      await quickTap(tester, point);
      final hold = await tester.startGesture(point);
      await tester.pump(const Duration(milliseconds: 120));
      deps.controller.updateState((state) => state.copyWith(selection: seededRangeSelection()));
      await tester.pump();

      await hold.up();
      await tester.pump(const Duration(milliseconds: 200));

      expect(find.byType(SelectionContextMenu), findsOneWidget);
    });

    testWidgets('double-tap drag selection shows context menu after release', (tester) async {
      final deps = _PageListHarnessDeps.create(immediateSettledTicker: true);
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final point = interactionPoint(tester, deps) + const Offset(0, 140);

      await quickTap(tester, point);
      final drag = await tester.startGesture(point);
      await tester.pump(const Duration(milliseconds: 120));
      deps.controller.updateState((state) => state.copyWith(selection: seededRangeSelection()));
      await tester.pump();
      await drag.moveBy(const Offset(0, -90));
      await tester.pump();
      await drag.up();
      await tester.pump(const Duration(milliseconds: 200));

      expect(find.byType(SelectionContextMenu), findsOneWidget);
    });

    testWidgets('double-tap word selection shows context menu when selection materializes', (tester) async {
      final deps = _PageListHarnessDeps.create(immediateSettledTicker: true);
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final point = interactionPoint(tester, deps);
      await quickTap(tester, point);
      await quickTap(tester, point);
      deps.controller.updateState((state) => state.copyWith(selection: seededRangeSelection()));
      await tester.pump(const Duration(milliseconds: 200));

      expect(find.byType(SelectionContextMenu), findsOneWidget);
    });

    testWidgets('selection dnd locks pan and unlocks after session end', (tester) async {
      final deps = _PageListHarnessDeps.create(selectionHit: true, clipboardData: {'text': 'hello'});
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final interaction = interactionControllerOf(tester);
      final point = interactionPoint(tester, deps);
      final resolved = interaction.resolveSelectionDrag(point);
      expect(resolved, isNotNull);

      interaction.startLocalDndSession(resolved!);
      await tester.pump();
      expect(deps.interactionState.snapshot().mode, InteractionMode.dndLocal);

      final beforeLocked = deps.verticalScrollController.offset;
      await dragPageList(tester, deps, dy: -260);
      final duringLocked = deps.verticalScrollController.offset;
      expect((duringLocked - beforeLocked).abs(), lessThan(0.1));

      interaction.endDndSession();
      await tester.pump();
      expect(deps.interactionState.snapshot().mode, InteractionMode.idle);

      final beforePan = deps.verticalScrollController.offset;
      await dragPageList(tester, deps, dy: -260);
      final afterPan = deps.verticalScrollController.offset;
      expect(afterPan, greaterThan(beforePan + 1));
    });
  });
}
