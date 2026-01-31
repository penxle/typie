import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/controller/focus_controller.dart';
import 'package:typie/screens/native_editor/controller/scroll_behavior.dart';
import 'package:typie/screens/native_editor/controller/ticker_loop.dart';
import 'package:typie/screens/native_editor/editor_input_view.dart';
import 'package:typie/screens/native_editor/external/models.dart';
import 'package:typie/screens/native_editor/fonts.dart';
import 'package:typie/screens/native_editor/handler/keyboard_handler.dart';
import 'package:typie/screens/native_editor/state/editor_state.dart';
import 'package:typie/screens/native_editor/toolbar/floating/floating.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/toolbar/toolbar.dart';
import 'package:typie/screens/native_editor/upload_manager.dart';
import 'package:typie/screens/native_editor/view/page_list.dart';
import 'package:typie/services/keyboard.dart';

class EditorView extends HookWidget {
  const EditorView({
    required this.editor,
    required this.fontManager,
    required this.width,
    required this.height,
    super.key,
  });

  final NativeEditor editor;
  final EditorFontManager? fontManager;
  final double width;
  final double height;

  @override
  Widget build(BuildContext context) {
    final controller = useMemoized(() => EditorController(editor: editor, fontManager: fontManager), [editor]);
    useEffect(() => controller.dispose, [controller]);

    final tickerProvider = useSingleTickerProvider();
    final scrollController = useScrollController();
    final inputKey = useMemoized(GlobalKey<EditorInputViewState>.new);
    final inputCausedCursorChange = useRef(false);

    final keyboardHeight = useValueNotifier<double>(0);
    final isKeyboardVisible = useValueNotifier<bool>(false);
    final keyboardType = useValueNotifier<KeyboardType>(KeyboardType.software);
    final bottomToolbarMode = useValueNotifier<BottomToolbarMode>(BottomToolbarMode.hidden);
    final secondaryToolbarMode = useValueNotifier<SecondaryToolbarMode>(SecondaryToolbarMode.hidden);

    final uniformMarks = useValueNotifier<List<Map<String, dynamic>>>([]);
    final mixedMarks = useValueNotifier<List<String>>([]);
    final selectionStats = useValueNotifier<Map<String, dynamic>>({});

    final externalElements = useValueNotifier<List<ExternalElement>>([]);
    final uploadManager = useMemoized(UploadManager.new);

    useEffect(() => uploadManager.dispose, []);

    final focusController = useMemoized(
      () => EditorFocusController(inputKey: inputKey, onFocusChanged: controller.setFocused),
      [controller],
    );

    final keyboardHandler = useMemoized(() => EditorKeyboardHandler(dispatch: controller.dispatch), [controller]);

    final widthRef = useRef(width);
    final heightRef = useRef(height);
    widthRef.value = width;
    heightRef.value = height;

    final tickerLoop = useMemoized(
      () => EditorTickerLoop(
        controller: controller,
        tickerProvider: tickerProvider,
        getSize: () => (widthRef.value, heightRef.value),
      ),
      [controller],
    );

    useEffect(() {
      tickerLoop.start();
      return tickerLoop.dispose;
    }, [tickerLoop]);

    final keyboard = useService<Keyboard>();

    useEffect(() {
      final subscription = keyboard.onHeightChange.listen((double height) {
        if (height > 0) {
          keyboardHeight.value = height;
          bottomToolbarMode.value = BottomToolbarMode.hidden;
        }
        isKeyboardVisible.value = height > 0;
      });
      return subscription.cancel;
    }, []);

    useEffect(() {
      final subscription = keyboard.onTypeChange.listen((KeyboardType type) {
        keyboardType.value = type;
      });
      return subscription.cancel;
    }, []);

    useEffect(() {
      bool onKeyEvent(KeyEvent event) {
        if (!focusController.isActive) {
          return false;
        }
        return keyboardHandler.handleKeyEvent(event);
      }

      HardwareKeyboard.instance.addHandler(onKeyEvent);
      return () => HardwareKeyboard.instance.removeHandler(onKeyEvent);
    }, []);

    final state = useListenable(controller);
    final currentLayout = state.state.layout;
    final cursor = state.state.cursor;

    useEffect(() {
      uniformMarks.value = state.state.uniformMarks;
      mixedMarks.value = state.state.mixedMarks;
      selectionStats.value = state.state.selectionStats;
      externalElements.value = state.state.externalElements;
      return null;
    }, [state.state.uniformMarks, state.state.mixedMarks, state.state.selectionStats, state.state.externalElements]);

    final viewKeyboardHeight = MediaQuery.viewInsetsOf(context).bottom;

    useEffect(() {
      if (cursor != null && cursor.show) {
        focusController.updateCursor(cursor.x, cursor.y, cursor.height);

        if (viewKeyboardHeight > 0 && currentLayout != null) {
          EditorScrollBehavior(
            scrollController: scrollController,
            viewportHeight: height,
            viewKeyboardHeight: viewKeyboardHeight,
          ).scrollToCursor(cursor, currentLayout);
        }
      }

      if (inputCausedCursorChange.value) {
        inputCausedCursorChange.value = false;
      } else {
        focusController.resetInputContext();
      }
      return null;
    }, [cursor, viewKeyboardHeight]);

    if (currentLayout == null) {
      return const Center(child: CircularProgressIndicator());
    }

    return NativeEditorToolbarScope(
      keyboardHeight: keyboardHeight,
      isKeyboardVisible: isKeyboardVisible,
      keyboardType: keyboardType,
      bottomToolbarMode: bottomToolbarMode,
      secondaryToolbarMode: secondaryToolbarMode,
      uniformMarks: uniformMarks,
      mixedMarks: mixedMarks,
      selectionStats: selectionStats,
      externalElements: externalElements,
      uploadManager: uploadManager,
      dispatch: controller.dispatch,
      requestFocus: focusController.requestFocus,
      clearFocus: focusController.clearFocus,
      child: Column(
        children: [
          Expanded(
            child: Stack(
              children: [
                PageList(
                  editor: editor,
                  layout: currentLayout,
                  cursor: cursor,
                  isFocused: state.state.isFocused,
                  isSelecting: state.state.isSelecting,
                  renderVersion: state.state.renderVersion,
                  scrollController: scrollController,
                  viewKeyboardHeight: viewKeyboardHeight,
                  onOpenInput: focusController.openInput,
                  onSelectionStart: () => controller.setSelecting(true),
                  onSelectionEnd: () => controller.setSelecting(false),
                ),
                Positioned.fill(
                  child: EditorInputView(
                    key: inputKey,
                    onInsertText: (text) {
                      inputCausedCursorChange.value = true;
                      controller.dispatch({'type': 'input', 'text': text});
                    },
                    onDeleteBackward: () {
                      inputCausedCursorChange.value = true;
                      controller.dispatch({'type': 'deleteBackward'});
                    },
                    onSetMarkedText: (text) {
                      inputCausedCursorChange.value = true;
                      controller.dispatch({'type': 'compositionUpdate', 'text': text});
                    },
                    onUnmarkText: () {
                      inputCausedCursorChange.value = true;
                      controller.dispatch({'type': 'commitPreedit'});
                    },
                    onCancelMarkedText: () {
                      inputCausedCursorChange.value = true;
                      controller.dispatch({'type': 'compositionEnd'});
                    },
                    onPerformAction: (action) {
                      if (action == 'newline') {
                        controller.dispatch({'type': 'insertNewline'});
                      }
                    },
                    onShortcut: (action) {
                      controller.dispatch({'type': action});
                    },
                  ),
                ),
                const Positioned(bottom: 20, right: 20, child: NativeEditorFloatingToolbar()),
                Positioned(
                  bottom: 20,
                  left: 0,
                  right: 0,
                  child: Center(child: _FontLoadingIndicator(isLoading: state.state.isLoadingFonts)),
                ),
              ],
            ),
          ),
          const NativeEditorToolbar(),
        ],
      ),
    );
  }
}

class _FontLoadingIndicator extends StatelessWidget {
  const _FontLoadingIndicator({required this.isLoading});

  final bool isLoading;

  @override
  Widget build(BuildContext context) {
    return AnimatedSlide(
      offset: isLoading ? Offset.zero : const Offset(0, 0.5),
      duration: const Duration(milliseconds: 150),
      child: AnimatedOpacity(
        opacity: isLoading ? 1.0 : 0.0,
        duration: const Duration(milliseconds: 150),
        child: IgnorePointer(
          ignoring: !isLoading,
          child: Container(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
            decoration: BoxDecoration(
              color: context.colors.surfaceSubtle,
              border: Border.all(color: context.colors.borderStrong),
              borderRadius: BorderRadius.circular(8),
            ),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                SizedBox(
                  width: 12,
                  height: 12,
                  child: CircularProgressIndicator(strokeWidth: 1, color: context.colors.textSubtle),
                ),
                const Gap(8),
                Text('폰트 로드 중...', style: TextStyle(fontSize: 13, color: context.colors.textDefault)),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
