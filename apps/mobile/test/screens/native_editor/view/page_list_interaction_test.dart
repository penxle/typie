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
import 'package:typie/screens/native_editor/view/input.dart';
import 'package:typie/screens/native_editor/view/interaction/input.dart';
import 'package:typie/screens/native_editor/view/interaction/mode.dart';
import 'package:typie/screens/native_editor/view/interaction/state.dart';
import 'package:typie/screens/native_editor/view/pages.dart';
import 'package:typie/screens/native_editor/view/scope.dart';
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

  factory _PageListHarnessDeps.create() {
    final editor = NativeEditor.test();
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
    final ticker = EditorTicker(getController: readController, tickerProvider: const TestVSync());

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

  Widget build() {
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
          child: ContentScope(
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
          ),
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
    editor.dispose();
  }
}

void main() {
  Future<double> dragPageList(WidgetTester tester, _PageListHarnessDeps deps, {double dy = -220}) async {
    await tester.drag(find.byType(PageList), Offset(0, dy));
    await tester.pump();
    return deps.verticalScrollController.offset;
  }

  void expectScrollLocked(double before, double after) {
    expect((after - before).abs(), lessThan(0.1));
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
    testWidgets('stale dnd lock recovers on pan attempt (image-drag failure regression)', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final beforeStaleLock = await dragPageList(tester, deps, dy: -280);
      expect(beforeStaleLock, greaterThan(0));

      deps.interactionState.handle(const DndStartInput(local: true));
      await tester.pump();
      expect(deps.interactionState.snapshot().isDndActive, isTrue);

      final afterRecovery = await dragPageList(tester, deps, dy: -280);
      expect(afterRecovery, greaterThan(beforeStaleLock + 1));
      expect(deps.interactionState.snapshot().isDndActive, isFalse);
    });

    testWidgets('pinch end resumes single-pointer pan', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final target = find.byType(PageList);
      final center = tester.getCenter(target);
      final g1 = await tester.startGesture(center + const Offset(-40, 0), pointer: 1);
      final g2 = await tester.startGesture(center + const Offset(40, 0), pointer: 2);
      await tester.pump();

      await g1.moveTo(center + const Offset(-90, 0));
      await g2.moveTo(center + const Offset(90, 0));
      await tester.pump();
      expect(deps.interactionState.snapshot().isPinching, isTrue);

      await g2.up();
      await tester.pump();
      expect(deps.interactionState.snapshot().isPinching, isFalse);

      final beforePan = deps.verticalScrollController.offset;
      await g1.moveBy(const Offset(0, -180));
      await tester.pump();
      await g1.up();
      await tester.pump();
      expect(deps.verticalScrollController.offset, greaterThan(beforePan + 1));
    });

    testWidgets('long-press selecting mode blocks pan until end', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      final beforeLock = await dragPageList(tester, deps);

      deps.interactionState.handle(const LongPressStartInput());
      await tester.pump();

      final duringLock = await dragPageList(tester, deps);
      expectScrollLocked(beforeLock, duringLock);

      deps.interactionState.handle(const LongPressEndInput());
      await tester.pump();

      final afterUnlock = await dragPageList(tester, deps);
      expect(afterUnlock, greaterThan(beforeLock + 1));
    });

    testWidgets('dnd leave clears external mode and pan remains available', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      deps.interactionState.handle(const DndEnterInput());
      await tester.pump();
      expect(deps.interactionState.snapshot().mode, InteractionMode.dndExternal);

      deps.interactionState.handle(const DndLeaveInput());
      await tester.pump();
      expect(deps.interactionState.snapshot().mode, InteractionMode.idle);

      final beforePan = deps.verticalScrollController.offset;
      final afterUnlock = await dragPageList(tester, deps, dy: -260);
      expect(afterUnlock, greaterThan(beforePan + 1));
    });

    testWidgets('auxiliary mode stays locked through update and unlocks on end', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      deps.interactionState.handle(const AuxiliaryGestureStartInput(kind: AuxiliaryGestureKind.tableColumnResize));
      await tester.pump();
      expect(deps.interactionState.snapshot().mode, InteractionMode.auxiliaryGesture);

      final beforeLocked = deps.verticalScrollController.offset;
      final duringStartLocked = await dragPageList(tester, deps);
      expectScrollLocked(beforeLocked, duringStartLocked);

      deps.interactionState.handle(const AuxiliaryGestureUpdateInput(kind: AuxiliaryGestureKind.tableColumnResize));
      await tester.pump();
      final duringUpdateLocked = await dragPageList(tester, deps);
      expectScrollLocked(duringStartLocked, duringUpdateLocked);

      deps.interactionState.handle(const AuxiliaryGestureEndInput());
      await tester.pump();
      expect(deps.interactionState.snapshot().mode, InteractionMode.idle);

      final afterUnlock = await dragPageList(tester, deps);
      expect(afterUnlock, greaterThan(duringUpdateLocked + 1));
    });

    testWidgets('pointer cancel releases text-handle selecting lock', (tester) async {
      final deps = _PageListHarnessDeps.create();
      addTearDown(deps.dispose);
      await tester.pumpWidget(deps.build());
      await tester.pumpAndSettle();

      deps.interactionState.handle(const TextHandleDragStartInput());
      await tester.pump();
      expect(deps.interactionState.snapshot().mode, InteractionMode.textHandleDragging);

      final beforeLocked = deps.verticalScrollController.offset;
      final duringLocked = await dragPageList(tester, deps, dy: -240);
      expectScrollLocked(beforeLocked, duringLocked);

      deps.interactionState.handle(const PointerCancelInput(pointer: 42));
      await tester.pump();
      expect(deps.interactionState.snapshot().mode, InteractionMode.idle);

      final afterUnlock = await dragPageList(tester, deps, dy: -240);
      expect(afterUnlock, greaterThan(duringLocked + 1));
    });
  });
}
