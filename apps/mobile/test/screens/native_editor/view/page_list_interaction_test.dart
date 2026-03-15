import 'package:flutter/gestures.dart';
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
import 'package:typie/screens/native_editor/view/interaction/state.dart';
import 'package:typie/screens/native_editor/view/pages.dart';
import 'package:typie/screens/native_editor/view/scope.dart';
import 'package:typie/screens/native_editor/view/selection.dart';
import 'package:typie/screens/native_editor/view/visible_area.dart';
import 'package:typie/service.dart';
import 'package:typie/services/keyboard.dart';
import 'package:typie/services/preference.dart';
import 'package:typie/styles/theme_data.dart';

class _TestPref implements Pref {
  @override
  String? siteId = 'test-site';

  @override
  bool devMode = false;

  @override
  bool typewriterEnabled = false;

  @override
  double typewriterPosition = 0.5;

  @override
  bool lineHighlightEnabled = false;

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
    required this.visibleArea,
    required this.longPressPosition,
    required this.handleDragPosition,
    required this.titleAreaHeight,
    required this.viewportTopInset,
    required this.viewportSize,
    required this.title,
    required this.subtitle,
    required this.pendingScroll,
    required this.pendingScrollPageIdx,
    required this.visualSyncPageIdx,
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
    Map<String, dynamic>? clipboardData,
    bool immediateSettledTicker = false,
  }) {
    final editor = NativeEditor.test(selectionHit: selectionHit, clipboardData: clipboardData);
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
    final viewportTopInset = ValueNotifier<double>(0);
    final viewportSize = ValueNotifier<Size>(Size.zero);
    final displayZoom = ValueNotifier<double>(1);
    final renderZoom = ValueNotifier<double>(1);

    final inputController = InputController(
      inputKey: GlobalKey<EditorTextInputState>(),
      dispatch: controller.dispatch,
      editor: editor,
      onFocusChanged: controller.setFocused,
      scrollIntoView: controller.scrollIntoView,
      getBottomToolbarMode: _hiddenBottomToolbarMode,
      getEditorSelection: () => null,
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
      visibleArea: VisibleEditorAreaNotifier(
        viewportSize: viewportSize,
        topInset: viewportTopInset,
        bottomInset: controller.sheetBottomInset,
      ),
      longPressPosition: ValueNotifier<Offset?>(null),
      handleDragPosition: ValueNotifier<Offset?>(null),
      titleAreaHeight: titleAreaHeight,
      viewportTopInset: viewportTopInset,
      viewportSize: viewportSize,
      title: ValueNotifier<String>(''),
      subtitle: ValueNotifier<String>(''),
      pendingScroll: ValueNotifier<VoidCallback?>(null),
      pendingScrollPageIdx: ValueNotifier<int?>(null),
      visualSyncPageIdx: ValueNotifier<int?>(null),
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
  final VisibleEditorAreaNotifier visibleArea;
  final ValueNotifier<Offset?> longPressPosition;
  final ValueNotifier<Offset?> handleDragPosition;
  final ValueNotifier<double> titleAreaHeight;
  final ValueNotifier<double> viewportTopInset;
  final ValueNotifier<Size> viewportSize;
  final ValueNotifier<String> title;
  final ValueNotifier<String> subtitle;
  final ValueNotifier<VoidCallback?> pendingScroll;
  final ValueNotifier<int?> pendingScrollPageIdx;
  final ValueNotifier<int?> visualSyncPageIdx;
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

  Widget build({bool rebuildContentScopeOnControllerChange = false, TargetPlatform platform = TargetPlatform.android}) {
    EditorInteractionController.debugIsAndroidOverride = switch (platform) {
      TargetPlatform.android => true,
      TargetPlatform.iOS => false,
      _ => null,
    };

    Widget buildContentScope() {
      return ContentScope(
        controller: controller,
        ticker: ticker,
        verticalScrollController: verticalScrollController,
        horizontalScrollController: horizontalScrollController,
        visibleArea: visibleArea,
        inputController: inputController,
        viewportSize: viewportSize,
        longPressPosition: longPressPosition,
        handleDragPosition: handleDragPosition,
        titleAreaHeight: titleAreaHeight,
        viewportTopInset: viewportTopInset,
        title: title,
        subtitle: subtitle,
        onTitleChanged: (_) {},
        onSubtitleChanged: (_) {},
        titleFocusNode: titleFocusNode,
        subtitleFocusNode: subtitleFocusNode,
        pendingScroll: pendingScroll,
        pendingScrollPageIdx: pendingScrollPageIdx,
        visualSyncPageIdx: visualSyncPageIdx,
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
        child: LayoutBuilder(
          builder: (context, constraints) {
            final nextViewportSize = Size(constraints.maxWidth, constraints.maxHeight);
            if (viewportSize.value != nextViewportSize) {
              WidgetsBinding.instance.addPostFrameCallback((_) {
                if (viewportSize.value != nextViewportSize) {
                  viewportSize.value = nextViewportSize;
                }
              });
            }
            return const PageList();
          },
        ),
      );
    }

    final content = rebuildContentScopeOnControllerChange
        ? AnimatedBuilder(animation: controller, builder: (_, _) => buildContentScope())
        : buildContentScope();

    return MaterialApp(
      theme: lightTheme.copyWith(platform: platform),
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
          reconcileInput: inputController.invalidate,
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
    visibleArea.dispose();
    longPressPosition.dispose();
    handleDragPosition.dispose();
    titleAreaHeight.dispose();
    viewportTopInset.dispose();
    viewportSize.dispose();
    title.dispose();
    subtitle.dispose();
    pendingScroll.dispose();
    pendingScrollPageIdx.dispose();
    visualSyncPageIdx.dispose();
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

  EditorSelection seededAndroidWordSelection() {
    return const EditorSelection(
      collapsed: false,
      cmp: 1,
      anchorBounds: SelectionEndpointBounds(pageIdx: 0, x: 220, y: 340, width: 1, height: 20),
      headBounds: SelectionEndpointBounds(pageIdx: 0, x: 280, y: 340, width: 1, height: 20),
      range: {
        'anchor': {'nodeId': 'n2', 'offset': 11, 'affinity': 'forward'},
        'head': {'nodeId': 'n2', 'offset': 15, 'affinity': 'forward'},
      },
    );
  }

  EditorSelection seededCollapsedSelection() {
    return const EditorSelection(
      range: {
        'anchor': {'nodeId': 'n3', 'offset': 7, 'affinity': 'forward'},
        'head': {'nodeId': 'n3', 'offset': 7, 'affinity': 'forward'},
      },
    );
  }

  CursorInfo seededCursor() {
    return const CursorInfo(pageIdx: 0, x: 160, y: 360, height: 20, visible: true);
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
    EditorInteractionController.debugIsAndroidOverride = null;
    if (serviceLocator.isRegistered<Pref>()) {
      await serviceLocator.unregister<Pref>();
    }
    serviceLocator.registerSingleton<Pref>(_TestPref());
  });

  tearDown(() async {
    EditorInteractionController.debugIsAndroidOverride = null;
    if (serviceLocator.isRegistered<Pref>()) {
      await serviceLocator.reset();
    }
  });

  group('PageList interaction regression', () {
    test('selection handle semantic clearSelectionHandleState clears handle drag state', () {
      const anchor = SelectionEndpointBounds(pageIdx: 0, x: 1, y: 2, width: 3, height: 4);
      final semantic = SelectionHandleSemantic()
        ..beginPendingSelectionHandleDrag(type: SelectionHandleType.to, touchPosition: const Offset(12, 8))
        ..beginSelectionHandleDrag(
          type: SelectionHandleType.to,
          touchPosition: const Offset(12, 8),
          handleScreenPosition: const Offset(10, 10),
          anchorHandle: anchor,
        )
        ..clearSelectionHandleState();

      expect(semantic.hasSelectionHandleDrag, isFalse);
      expect(semantic.hasPendingSelectionHandleDrag, isFalse);
      expect(semantic.pointerDownTouchPosition, isNull);
      expect(semantic.selectionHandleDragContext(), isNull);
      expect(semantic.hasAnyHandleDrag, isFalse);
    });

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

    testWidgets('selection handle auto-scroll uses visible top edge below header inset', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      deps.viewportTopInset.value = 80;
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      deps.controller.updateState(
        (state) => state.copyWith(
          selection: const EditorSelection(
            collapsed: false,
            cmp: 1,
            anchorBounds: SelectionEndpointBounds(pageIdx: 0, x: 120, y: 520, width: 1, height: 20),
            headBounds: SelectionEndpointBounds(pageIdx: 0, x: 180, y: 560, width: 1, height: 20),
            range: {
              'anchor': {'nodeId': 'n1', 'offset': 3, 'affinity': 'forward'},
              'head': {'nodeId': 'n1', 'offset': 12, 'affinity': 'forward'},
            },
          ),
        ),
      );
      await tester.pump();

      deps.verticalScrollController.jumpTo(400);
      await tester.pump();

      final handles = find.byType(SelectionHandle);
      expect(handles, findsWidgets);

      final interaction = interactionControllerOf(tester);
      final listTopLeft = tester.getTopLeft(find.byType(PageList));
      final size = tester.getSize(find.byType(PageList));
      final handleCenter = tester.getCenter(handles.last);
      final before = deps.verticalScrollController.offset;

      interaction
        ..onHandleDragDown(SelectionHandleType.to, DragDownDetails(globalPosition: handleCenter))
        ..onHandleDragStart(SelectionHandleType.to, DragStartDetails(globalPosition: handleCenter));
      await tester.pump();
      interaction.onHandleDragUpdate(
        SelectionHandleType.to,
        DragUpdateDetails(
          globalPosition: listTopLeft + Offset(size.width / 2, deps.viewportTopInset.value + 10),
          delta: const Offset(0, -70),
        ),
      );
      await tester.pump(const Duration(milliseconds: 250));
      interaction.onHandleDragEnd(SelectionHandleType.to, DragEndDetails());
      await tester.pump();

      final after = deps.verticalScrollController.offset;
      expect(after, lessThan(before - 1));
    });

    testWidgets('onPan path stays blocked while double-tap drag is pending and dragging', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final interaction = interactionControllerOf(tester);
      final point = interactionPoint(tester, deps) + const Offset(0, 140);
      final local = tester.getTopLeft(find.byType(PageList));
      final localPoint = point - local;

      await quickTap(tester, point);
      final hold = await tester.startGesture(point);
      await tester.pump(const Duration(milliseconds: 120));

      final beforePendingPan = deps.verticalScrollController.offset;
      interaction
        ..onPanStart(DragStartDetails(globalPosition: point, localPosition: localPoint))
        ..onPanUpdate(
          DragUpdateDetails(
            globalPosition: point + const Offset(0, -140),
            localPosition: localPoint + const Offset(0, -140),
            delta: const Offset(0, -140),
          ),
        );
      await tester.pump();
      final afterPendingPan = deps.verticalScrollController.offset;
      expect((afterPendingPan - beforePendingPan).abs(), lessThan(0.1));

      deps.controller.updateState((state) => state.copyWith(selection: seededRangeSelection()));
      await tester.pump();
      await hold.moveBy(const Offset(0, -90));
      await tester.pump();
      expect(deps.interactionState.snapshot().mode, InteractionMode.doubleTapSelecting);

      final beforeDraggingPan = deps.verticalScrollController.offset;
      interaction
        ..onPanStart(
          DragStartDetails(
            globalPosition: point + const Offset(0, -90),
            localPosition: localPoint + const Offset(0, -90),
          ),
        )
        ..onPanUpdate(
          DragUpdateDetails(
            globalPosition: point + const Offset(0, -210),
            localPosition: localPoint + const Offset(0, -210),
            delta: const Offset(0, -120),
          ),
        )
        ..onPanEnd(DragEndDetails());
      await tester.pump();
      final afterDraggingPan = deps.verticalScrollController.offset;
      expect((afterDraggingPan - beforeDraggingPan).abs(), lessThan(0.1));

      await hold.up();
      await tester.pump();
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

    testWidgets('double-tap on existing selection does not queue scroll intent', (tester) async {
      final deps = _PageListHarnessDeps.create(selectionHit: true);
      addTearDown(deps.dispose);
      deps.controller.updateState(
        (state) => state.copyWith(
          selection: const EditorSelection(
            collapsed: false,
            cmp: 1,
            range: {
              'anchor': {'nodeId': 'n1', 'offset': 3, 'affinity': 'forward'},
              'head': {'nodeId': 'n1', 'offset': 8, 'affinity': 'forward'},
            },
          ),
        ),
      );
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final point = interactionPoint(tester, deps);

      await quickTap(tester, point);
      deps.editor.clearTestDispatchedMessages();
      await quickTap(tester, point);
      await tester.pump();

      expect(deps.controller.pendingScrollMode, isNull);
      expect(deps.pendingScroll.value, isNull);

      final pointerDowns = deps.editor.testDispatchedMessages.where((message) => message['type'] == 'pointerDown');
      expect(pointerDowns.where((message) => message['clickCount'] == 2).length, 1);
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

    testWidgets('selection handles resync after render completes without cursor', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);

      final initialRenderVersion = Object();
      final updatedRenderVersion = Object();

      deps.controller.updateState(
        (state) => state.copyWith(selection: seededRangeSelection(), renderVersion: initialRenderVersion),
      );
      deps.presentedViewport.value = PresentedViewport.base(cursor: null, renderVersion: initialRenderVersion);

      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final handles = find.byType(SelectionHandle);
      expect(handles, findsWidgets);

      final before = tester.getTopLeft(handles.last);

      deps.controller.updateState(
        (state) => state.copyWith(
          selection: const EditorSelection(
            collapsed: false,
            cmp: 1,
            anchorBounds: SelectionEndpointBounds(pageIdx: 0, x: 120, y: 320, width: 1, height: 20),
            headBounds: SelectionEndpointBounds(pageIdx: 0, x: 180, y: 240, width: 1, height: 20),
            range: {
              'anchor': {'nodeId': 'n1', 'offset': 3, 'affinity': 'forward'},
              'head': {'nodeId': 'n1', 'offset': 12, 'affinity': 'forward'},
            },
          ),
          renderVersion: updatedRenderVersion,
        ),
      );
      await tester.pump();

      final held = tester.getTopLeft(handles.last);
      expect((held.dy - before.dy).abs(), lessThan(1));

      deps.presentedViewport.value = PresentedViewport.base(cursor: null, renderVersion: updatedRenderVersion);
      await tester.pump();

      final after = tester.getTopLeft(handles.last);
      expect(after.dy, lessThan(before.dy - 40));
    });

    testWidgets('double-tap selection handle drag locks pan before first extension', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final point = interactionPoint(tester, deps);
      await quickTap(tester, point);
      await quickTap(tester, point);
      deps.controller.updateState((state) => state.copyWith(selection: seededRangeSelection()));
      await tester.pump();

      final handles = find.byType(SelectionHandle);
      expect(handles, findsWidgets);

      deps.editor.clearTestDispatchedMessages();
      final beforePan = deps.verticalScrollController.offset;

      final drag = await tester.startGesture(tester.getCenter(handles.last));
      await tester.pump();
      await drag.moveBy(const Offset(0, -28));
      await tester.pump();

      final afterFirstMovePan = deps.verticalScrollController.offset;
      final modeAfterFirstMove = deps.interactionState.snapshot().mode;

      await drag.moveBy(const Offset(0, -52));
      await tester.pump();
      await drag.up();
      await tester.pump();

      final extendEvents = deps.editor.testDispatchedMessages
          .where((message) => message['type'] == 'extendSelectionTo')
          .toList();

      expect((afterFirstMovePan - beforePan).abs(), lessThan(0.1));
      expect(modeAfterFirstMove, InteractionMode.selectionHandleDragging);
      expect(extendEvents, isNotEmpty);
    });

    testWidgets('android long-press move extends word selection and clears magnifier on release', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final start = interactionPoint(tester, deps);

      deps.editor.clearTestDispatchedMessages();
      final gesture = await tester.startGesture(start);
      await tester.pump(const Duration(milliseconds: 600));
      deps.controller.updateState((state) => state.copyWith(selection: seededAndroidWordSelection()));
      await tester.pump();

      await gesture.moveBy(const Offset(16, -6));
      await tester.pump();
      await gesture.moveBy(const Offset(18, -8));
      await tester.pump();

      final wordPointerDownCount = deps.editor.testDispatchedMessages
          .where((message) => message['type'] == 'pointerDown' && message['clickCount'] == 2)
          .length;
      final extendEvents = deps.editor.testDispatchedMessages
          .where((message) => message['type'] == 'extendSelectionTo')
          .toList();
      expect(wordPointerDownCount, equals(1));
      expect(extendEvents.length, greaterThanOrEqualTo(2));
      expect(deps.longPressPosition.value, isNotNull);

      await gesture.up();
      await tester.pump();
      expect(deps.longPressPosition.value, isNull);
      expect(deps.interactionState.snapshot().mode, InteractionMode.idle);
    });

    testWidgets('table-cell handle down without drag start does not block android long-press', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final interaction = interactionControllerOf(tester);
      final point = interactionPoint(tester, deps);

      expect(interaction.beginTableCellHandleDragDown(DragDownDetails(globalPosition: point)), isTrue);
      expect(deps.interactionState.snapshot().mode, InteractionMode.idle);

      expect(interaction.startLongPress(point), isTrue);
      expect(deps.interactionState.snapshot().mode, InteractionMode.longPressWordSelecting);

      expect(interaction.endLongPress(), isTrue);
      expect(deps.interactionState.snapshot().mode, InteractionMode.idle);
    });

    testWidgets('double-tap selecting mode rejects long-press at recognizer gate and command gate', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final interaction = interactionControllerOf(tester);
      final point = interactionPoint(tester, deps);
      final localPoint = interaction.viewportPositionFromGlobal(point);
      expect(localPoint, isNotNull);

      expect(interaction.startDoubleTapDrag(localPoint!), isTrue);
      expect(deps.interactionState.snapshot().mode, InteractionMode.doubleTapSelecting);
      expect(interaction.shouldRejectLongPress(point), isTrue);
      expect(interaction.startLongPress(point), isFalse);

      expect(interaction.endDoubleTapDrag(), isTrue);
      expect(deps.interactionState.snapshot().mode, InteractionMode.idle);
    });

    testWidgets('pointer scroll path is blocked while selecting mode is active', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final interaction = interactionControllerOf(tester);
      final point = interactionPoint(tester, deps);
      final localPoint = interaction.viewportPositionFromGlobal(point);
      expect(localPoint, isNotNull);

      expect(interaction.startDoubleTapDrag(localPoint!), isTrue);
      expect(deps.interactionState.snapshot().mode, InteractionMode.doubleTapSelecting);

      final before = deps.verticalScrollController.offset;
      interaction.onPointerSignal(PointerScrollEvent(position: point, scrollDelta: const Offset(0, 180)));
      await tester.pump();
      final after = deps.verticalScrollController.offset;
      expect((after - before).abs(), lessThan(0.1));

      expect(interaction.endDoubleTapDrag(), isTrue);
      expect(deps.interactionState.snapshot().mode, InteractionMode.idle);
    });

    testWidgets('pointer scroll path pans in idle mode and does not leave panning mode stuck', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final interaction = interactionControllerOf(tester);
      final point = interactionPoint(tester, deps);

      final before = deps.verticalScrollController.offset;
      interaction.onPointerSignal(PointerScrollEvent(position: point, scrollDelta: const Offset(0, 180)));
      await tester.pump();
      final after = deps.verticalScrollController.offset;

      expect(after, greaterThan(before + 1));
      expect(deps.interactionState.snapshot().mode, InteractionMode.idle);
    });

    testWidgets('ios long-press move keeps updating cursor and clears magnifier on release', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build(platform: TargetPlatform.iOS));
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

    testWidgets('android long-press drag keeps dispatching selection extensions after first update', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final start = interactionPoint(tester, deps) + const Offset(0, 120);

      deps.editor.clearTestDispatchedMessages();
      final gesture = await tester.startGesture(start);
      await tester.pump(const Duration(milliseconds: 600));
      deps.controller.updateState((state) => state.copyWith(selection: seededAndroidWordSelection()));
      await tester.pump();

      await gesture.moveBy(const Offset(0, -20));
      await tester.pump();
      await gesture.moveBy(const Offset(0, -20));
      await tester.pump();
      await gesture.moveBy(const Offset(0, -20));
      await tester.pump();

      final extendEvents = deps.editor.testDispatchedMessages
          .where((message) => message['type'] == 'extendSelectionTo')
          .toList();
      final uniqueHeadY = extendEvents.map((message) => message['headY']).toSet();
      expect(extendEvents.length, greaterThanOrEqualTo(3));
      expect(uniqueHeadY.length, greaterThanOrEqualTo(2));

      await gesture.up();
      await tester.pump();
      expect(deps.longPressPosition.value, isNull);
    });

    testWidgets('ios long-press drag keeps dispatching cursor moves after first update', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build(platform: TargetPlatform.iOS));
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

    testWidgets('android long-press drag keeps dispatching extension after content-scope rebuild', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build(rebuildContentScopeOnControllerChange: true));
      await tester.pumpAndSettle();

      final start = interactionPoint(tester, deps) + const Offset(0, 120);

      deps.editor.clearTestDispatchedMessages();
      final gesture = await tester.startGesture(start);
      await tester.pump(const Duration(milliseconds: 600));
      deps.controller.updateState((state) => state.copyWith(selection: seededAndroidWordSelection()));
      await tester.pump();

      await gesture.moveBy(const Offset(0, -20));
      await tester.pump();
      await gesture.moveBy(const Offset(0, -20));
      await tester.pump();
      await gesture.moveBy(const Offset(0, -20));
      await tester.pump();

      final extendEvents = deps.editor.testDispatchedMessages
          .where((message) => message['type'] == 'extendSelectionTo')
          .toList();
      final uniqueHeadY = extendEvents.map((message) => message['headY']).toSet();
      expect(extendEvents.length, greaterThanOrEqualTo(3));
      expect(uniqueHeadY.length, greaterThanOrEqualTo(2));

      await gesture.up();
      await tester.pump();
      expect(deps.longPressPosition.value, isNull);
    });

    testWidgets('ios long-press drag keeps dispatching after content-scope rebuild', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build(rebuildContentScopeOnControllerChange: true, platform: TargetPlatform.iOS));
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

    testWidgets('pointer cancel clears long-press, double-tap drag, and handle-drag interaction state', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final point = interactionPoint(tester, deps) + const Offset(0, 120);
      final longPress = await tester.startGesture(point);
      await tester.pump(const Duration(milliseconds: 600));
      await longPress.moveBy(const Offset(0, -20));
      await tester.pump();
      expect(deps.longPressPosition.value, isNotNull);
      await longPress.cancel();
      await tester.pump();
      expect(deps.longPressPosition.value, isNull);
      expect(deps.interactionState.snapshot().mode, InteractionMode.idle);

      final beforePanAfterLongPressCancel = deps.verticalScrollController.offset;
      await dragPageList(tester, deps, dy: -240);
      final afterPanAfterLongPressCancel = deps.verticalScrollController.offset;
      expect(afterPanAfterLongPressCancel, greaterThan(beforePanAfterLongPressCancel + 1));

      final dragPoint = point + const Offset(0, 120);
      await quickTap(tester, dragPoint);
      final doubleTapDrag = await tester.startGesture(dragPoint);
      await tester.pump(const Duration(milliseconds: 120));
      deps.controller.updateState((state) => state.copyWith(selection: seededRangeSelection()));
      await tester.pump();
      await doubleTapDrag.moveBy(const Offset(0, -80));
      await tester.pump();
      expect(deps.interactionState.snapshot().mode, InteractionMode.doubleTapSelecting);
      expect(deps.handleDragPosition.value, isNotNull);

      await doubleTapDrag.cancel();
      await tester.pump();
      expect(deps.interactionState.snapshot().mode, InteractionMode.idle);
      expect(deps.longPressPosition.value, isNull);
      expect(deps.handleDragPosition.value, isNull);

      final beforePanAfterPointerCancel = deps.verticalScrollController.offset;
      await dragPageList(tester, deps, dy: -240);
      final afterPanAfterPointerCancel = deps.verticalScrollController.offset;
      expect(afterPanAfterPointerCancel, greaterThan(beforePanAfterPointerCancel + 1));
    });

    testWidgets('android long-press outside selection starts word selection and drag extension', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final point = interactionPoint(tester, deps);
      deps.editor.clearTestDispatchedMessages();
      final longPress = await tester.startGesture(point);
      await tester.pump(const Duration(milliseconds: 600));
      final freshWordSelection = seededAndroidWordSelection();
      deps.controller.updateState((state) => state.copyWith(selection: freshWordSelection));
      await tester.pump();
      await longPress.moveBy(const Offset(10, -10));
      await tester.pump();
      await longPress.up();
      await tester.pump();

      final pointerDowns = deps.editor.testDispatchedMessages
          .where((message) => message['type'] == 'pointerDown' && message['clickCount'] == 2)
          .toList();
      final extendEvents = deps.editor.testDispatchedMessages
          .where((message) => message['type'] == 'extendSelectionTo')
          .toList();
      expect(pointerDowns.length, equals(1));
      expect(extendEvents, isNotEmpty);
      expect(extendEvents.last['doubleTapInitialRange'], equals(freshWordSelection.range));
    });

    testWidgets('android long-press with existing selection waits for new word selection context', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      deps.controller.updateState((state) => state.copyWith(selection: seededRangeSelection()));
      await tester.pump();

      final point = interactionPoint(tester, deps);
      deps.editor.clearTestDispatchedMessages();

      final longPress = await tester.startGesture(point);
      await tester.pump(const Duration(milliseconds: 600));
      await longPress.moveBy(const Offset(8, -8));
      await tester.pump();

      final beforeFreshSelection = deps.editor.testDispatchedMessages
          .where((message) => message['type'] == 'extendSelectionTo')
          .toList();
      expect(beforeFreshSelection, isEmpty);

      final freshWordSelection = seededAndroidWordSelection();
      deps.controller.updateState((state) => state.copyWith(selection: freshWordSelection));
      await tester.pump();

      await longPress.moveBy(const Offset(8, -8));
      await tester.pump();
      await longPress.up();
      await tester.pump();

      final afterFreshSelection = deps.editor.testDispatchedMessages
          .where((message) => message['type'] == 'extendSelectionTo')
          .toList();
      expect(afterFreshSelection, isNotEmpty);
      expect(afterFreshSelection.last['doubleTapInitialRange'], equals(freshWordSelection.range));
    });

    testWidgets('android long-press on collapsed selection hit keeps cursor-move mode', (tester) async {
      final deps = _PageListHarnessDeps.create(selectionHit: true);
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      deps.controller.updateState(
        (state) => state.copyWith(selection: seededCollapsedSelection(), cursor: seededCursor()),
      );
      await tester.pump();

      final interactionController = interactionControllerOf(tester);
      final point = interactionPoint(tester, deps);
      deps.editor.clearTestDispatchedMessages();
      expect(interactionController.shouldRejectLongPress(point), isFalse);
      expect(interactionController.startLongPress(point), isTrue);
      expect(deps.interactionState.snapshot().mode, InteractionMode.longPressSelecting);

      final updatePoint = interactionController.viewportPositionFromGlobal(point + const Offset(12, -6));
      expect(updatePoint, isNotNull);
      interactionController.updateLongPress(updatePoint!);

      final wordPointerDowns = deps.editor.testDispatchedMessages
          .where((message) => message['type'] == 'pointerDown' && message['clickCount'] == 2)
          .toList();
      expect(wordPointerDowns, isEmpty);

      interactionController.endLongPress();
      expect(deps.interactionState.snapshot().mode, InteractionMode.idle);
    });

    testWidgets('android long-press on range selection hit is rejected at recognizer and start gate', (tester) async {
      final deps = _PageListHarnessDeps.create(selectionHit: true);
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      deps.controller.updateState((state) => state.copyWith(selection: seededRangeSelection(), cursor: seededCursor()));
      await tester.pump();

      final interactionController = interactionControllerOf(tester);
      final point = interactionPoint(tester, deps);

      expect(interactionController.shouldRejectLongPress(point), isTrue);
      expect(interactionController.startLongPress(point), isFalse);
      expect(deps.interactionState.snapshot().mode, InteractionMode.idle);
      expect(deps.longPressPosition.value, isNull);
    });

    testWidgets('active long-press session rejects re-entry at recognizer and start gate', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final interactionController = interactionControllerOf(tester);
      final point = interactionPoint(tester, deps);

      expect(interactionController.startLongPress(point), isTrue);
      expect(deps.interactionState.snapshot().isLongPressing, isTrue);

      expect(interactionController.shouldRejectLongPress(point), isTrue);
      expect(interactionController.startLongPress(point), isFalse);
      expect(deps.interactionState.snapshot().isLongPressing, isTrue);

      expect(interactionController.endLongPress(), isTrue);
      expect(deps.interactionState.snapshot().mode, InteractionMode.idle);
    });

    testWidgets('ios long-press outside selection keeps cursor move behavior', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build(platform: TargetPlatform.iOS));
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
      final extendEvents = deps.editor.testDispatchedMessages
          .where((message) => message['type'] == 'extendSelectionTo')
          .toList();
      expect(pointerDowns, isNotEmpty);
      expect(extendEvents, isEmpty);
    });

    testWidgets('ios long-press on range selection hit is rejected at recognizer and start gate', (tester) async {
      final deps = _PageListHarnessDeps.create(selectionHit: true);
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build(platform: TargetPlatform.iOS));
      await tester.pumpAndSettle();

      deps.controller.updateState((state) => state.copyWith(selection: seededRangeSelection(), cursor: seededCursor()));
      await tester.pump();

      final interactionController = interactionControllerOf(tester);
      final point = interactionPoint(tester, deps);

      expect(interactionController.shouldRejectLongPress(point), isTrue);
      expect(interactionController.startLongPress(point), isFalse);
      expect(deps.interactionState.snapshot().mode, InteractionMode.idle);
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

    testWidgets('context menu stays below visible top inset', (tester) async {
      final deps = _PageListHarnessDeps.create(immediateSettledTicker: true);
      addTearDown(deps.dispose);
      deps.viewportTopInset.value = 80;
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      deps.controller.updateState(
        (state) => state.copyWith(
          cursor: const CursorInfo(pageIdx: 0, x: 160, y: 20, height: 20, visible: true),
          selection: null,
        ),
      );
      await tester.pump();

      final point = interactionPoint(tester, deps);
      await quickTap(tester, point);
      await tester.pump(const Duration(milliseconds: 200));

      expect(find.byType(SelectionContextMenu), findsOneWidget);
      expect(tester.getTopLeft(find.text('붙여넣기')).dy, greaterThanOrEqualTo(deps.viewportTopInset.value));
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

    testWidgets('auxiliary gesture locks pan and unlocks immediately after end', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final interaction = interactionControllerOf(tester);
      final started = interaction.startAuxiliaryGesture(AuxiliaryGestureKind.tableColumnResize);
      expect(started, isTrue);
      expect(deps.interactionState.snapshot().mode, InteractionMode.auxiliaryGesture);

      final beforeLocked = deps.verticalScrollController.offset;
      await dragPageList(tester, deps, dy: -260);
      final afterLocked = deps.verticalScrollController.offset;
      expect((afterLocked - beforeLocked).abs(), lessThan(0.1));

      final ended = interaction.endAuxiliaryGesture();
      expect(ended, isTrue);
      await tester.pump();
      expect(deps.interactionState.snapshot().mode, InteractionMode.idle);

      final beforePan = deps.verticalScrollController.offset;
      await dragPageList(tester, deps, dy: -260);
      final afterPan = deps.verticalScrollController.offset;
      expect(afterPan, greaterThan(beforePan + 1));
    });

    testWidgets('selection dnd locks pan and unlocks after drag semantic end', (tester) async {
      final deps = _PageListHarnessDeps.create(selectionHit: true, clipboardData: {'text': 'hello'});
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final interaction = interactionControllerOf(tester);
      final point = interactionPoint(tester, deps);
      final resolved = interaction.resolveSelectionDrag(point);
      expect(resolved, isNotNull);

      interaction.startLocalDnd(resolved!);
      await tester.pump();
      expect(deps.interactionState.snapshot().mode, InteractionMode.dndLocal);

      final beforeLocked = deps.verticalScrollController.offset;
      await dragPageList(tester, deps, dy: -260);
      final duringLocked = deps.verticalScrollController.offset;
      expect((duringLocked - beforeLocked).abs(), lessThan(0.1));

      interaction.endDnd();
      await tester.pump();
      expect(deps.interactionState.snapshot().mode, InteractionMode.idle);

      final beforePan = deps.verticalScrollController.offset;
      await dragPageList(tester, deps, dy: -260);
      final afterPan = deps.verticalScrollController.offset;
      expect(afterPan, greaterThan(beforePan + 1));
    });

    testWidgets('dnd lock blocks both onPan and onPointer pan paths until drag semantic end', (tester) async {
      final deps = _PageListHarnessDeps.create(selectionHit: true, clipboardData: {'text': 'hello'});
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final interaction = interactionControllerOf(tester);
      final point = interactionPoint(tester, deps);
      final resolved = interaction.resolveSelectionDrag(point);
      expect(resolved, isNotNull);

      interaction.startLocalDnd(resolved!);
      await tester.pump();
      expect(deps.interactionState.snapshot().mode, InteractionMode.dndLocal);

      final localTopLeft = tester.getTopLeft(find.byType(PageList));
      final localPoint = point - localTopLeft;

      final beforeOnPan = deps.verticalScrollController.offset;
      interaction
        ..onPanStart(DragStartDetails(globalPosition: point, localPosition: localPoint))
        ..onPanUpdate(
          DragUpdateDetails(
            globalPosition: point + const Offset(0, -180),
            localPosition: localPoint + const Offset(0, -180),
            delta: const Offset(0, -180),
          ),
        )
        ..onPanEnd(DragEndDetails());
      await tester.pump();
      final afterOnPan = deps.verticalScrollController.offset;
      expect((afterOnPan - beforeOnPan).abs(), lessThan(0.1));

      final pointerDown = PointerDownEvent(pointer: 11, position: point);
      final pointerMove = PointerMoveEvent(pointer: 11, position: point + const Offset(0, -180));
      final pointerUp = PointerUpEvent(pointer: 11, position: point + const Offset(0, -180));

      final beforeOnPointer = deps.verticalScrollController.offset;
      interaction
        ..onPointerDown(pointerDown)
        ..onPointerMove(pointerMove)
        ..onPointerUp(pointerUp);
      await tester.pump();
      final afterOnPointer = deps.verticalScrollController.offset;
      expect((afterOnPointer - beforeOnPointer).abs(), lessThan(0.1));

      interaction.endDnd();
      await tester.pump();
      expect(deps.interactionState.snapshot().mode, InteractionMode.idle);

      final beforePan = deps.verticalScrollController.offset;
      await dragPageList(tester, deps, dy: -260);
      final afterPan = deps.verticalScrollController.offset;
      expect(afterPan, greaterThan(beforePan + 1));
    });

    testWidgets('dnd lock blocks pointer signal scroll path until drag semantic end', (tester) async {
      final deps = _PageListHarnessDeps.create(selectionHit: true, clipboardData: {'text': 'hello'});
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final interaction = interactionControllerOf(tester);
      final point = interactionPoint(tester, deps);
      final resolved = interaction.resolveSelectionDrag(point);
      expect(resolved, isNotNull);

      interaction.startLocalDnd(resolved!);
      await tester.pump();
      expect(deps.interactionState.snapshot().mode, InteractionMode.dndLocal);

      final beforeLocked = deps.verticalScrollController.offset;
      interaction.onPointerSignal(PointerScrollEvent(position: point, scrollDelta: const Offset(0, 220)));
      await tester.pump();
      final afterLocked = deps.verticalScrollController.offset;
      expect((afterLocked - beforeLocked).abs(), lessThan(0.1));

      interaction.endDnd();
      await tester.pump();
      expect(deps.interactionState.snapshot().mode, InteractionMode.idle);

      final beforeUnlocked = deps.verticalScrollController.offset;
      interaction.onPointerSignal(PointerScrollEvent(position: point, scrollDelta: const Offset(0, 220)));
      await tester.pump();
      final afterUnlocked = deps.verticalScrollController.offset;
      expect(afterUnlocked, greaterThan(beforeUnlocked + 1));
    });

    testWidgets('table-cell handle start is rejected without pending down', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final interaction = interactionControllerOf(tester);
      final point = interactionPoint(tester, deps);
      final viewportPosition = interaction.viewportPositionFromGlobal(point);
      expect(viewportPosition, isNotNull);

      final cellHandleDragPosition = ValueNotifier<Offset?>(null);
      addTearDown(cellHandleDragPosition.dispose);

      final started = interaction.startTableCellHandleDrag(
        anchorHandle: null,
        viewportPosition: viewportPosition,
        cellHandleDragPosition: cellHandleDragPosition,
      );

      expect(started, isFalse);
      expect(deps.interactionState.snapshot().mode, InteractionMode.idle);
      expect(cellHandleDragPosition.value, isNull);
    });

    testWidgets('table-cell handle down-start-end transitions mode and clears drag position', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final interaction = interactionControllerOf(tester);
      final point = interactionPoint(tester, deps);
      final viewportPosition = interaction.viewportPositionFromGlobal(point);
      expect(viewportPosition, isNotNull);

      final cellHandleDragPosition = ValueNotifier<Offset?>(null);
      addTearDown(cellHandleDragPosition.dispose);

      expect(interaction.beginTableCellHandleDragDown(DragDownDetails(globalPosition: point)), isTrue);
      expect(deps.interactionState.snapshot().mode, InteractionMode.idle);

      final started = interaction.startTableCellHandleDrag(
        anchorHandle: null,
        viewportPosition: viewportPosition,
        cellHandleDragPosition: cellHandleDragPosition,
      );
      expect(started, isTrue);
      expect(deps.interactionState.snapshot().mode, InteractionMode.tableCellHandleDragging);
      expect(cellHandleDragPosition.value, isNotNull);

      final ended = interaction.endTableCellHandleDrag(cellHandleDragPosition: cellHandleDragPosition);
      expect(ended, isTrue);
      expect(deps.interactionState.snapshot().mode, InteractionMode.idle);
      expect(cellHandleDragPosition.value, isNull);
    });

    testWidgets('table-cell handle end succeeds without external drag-position notifier', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final interaction = interactionControllerOf(tester);
      final point = interactionPoint(tester, deps);
      final viewportPosition = interaction.viewportPositionFromGlobal(point);
      expect(viewportPosition, isNotNull);

      final cellHandleDragPosition = ValueNotifier<Offset?>(null);

      expect(interaction.beginTableCellHandleDragDown(DragDownDetails(globalPosition: point)), isTrue);
      final started = interaction.startTableCellHandleDrag(
        anchorHandle: null,
        viewportPosition: viewportPosition,
        cellHandleDragPosition: cellHandleDragPosition,
      );
      expect(started, isTrue);
      expect(deps.interactionState.snapshot().mode, InteractionMode.tableCellHandleDragging);

      cellHandleDragPosition.dispose();
      expect(interaction.endTableCellHandleDrag(), isTrue);
      expect(deps.interactionState.snapshot().mode, InteractionMode.idle);
    });

    testWidgets('external dnd dropEnded ends session and unlocks pan', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final interaction = interactionControllerOf(tester)..onDropEnter(null);
      await tester.pump();
      expect(deps.interactionState.snapshot().mode, InteractionMode.dndExternal);

      final beforeLocked = deps.verticalScrollController.offset;
      await dragPageList(tester, deps, dy: -240);
      final afterLocked = deps.verticalScrollController.offset;
      expect((afterLocked - beforeLocked).abs(), lessThan(0.1));

      interaction.onDropEnded(null);
      await tester.pump();
      expect(deps.interactionState.snapshot().mode, InteractionMode.idle);

      final beforeUnlocked = deps.verticalScrollController.offset;
      await dragPageList(tester, deps, dy: -240);
      final afterUnlocked = deps.verticalScrollController.offset;
      expect(afterUnlocked, greaterThan(beforeUnlocked + 1));
    });

    testWidgets('local dnd leave and re-enter dispatches dragEnter', (tester) async {
      final deps = _PageListHarnessDeps.create(selectionHit: true, clipboardData: {'text': 'hello'});
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final interaction = interactionControllerOf(tester);
      final point = interactionPoint(tester, deps);
      final resolved = interaction.resolveSelectionDrag(point);
      expect(resolved, isNotNull);

      interaction.startLocalDnd(resolved!);
      await tester.pump();
      expect(deps.interactionState.snapshot().mode, InteractionMode.dndLocal);

      deps.editor.clearTestDispatchedMessages();
      interaction.onDropLeave(null);
      await tester.pump();
      expect(deps.interactionState.snapshot().mode, InteractionMode.dndLocal);

      interaction.onDropEnter(null);
      await tester.pump();

      final dragEnterEvents = deps.editor.testDispatchedMessages
          .where((message) => message['type'] == 'dragEnter')
          .toList();
      expect(dragEnterEvents.length, 1);
    });
  });
}
