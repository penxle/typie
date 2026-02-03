import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/screens/native_editor/controller/focus_controller.dart';
import 'package:typie/screens/native_editor/controller/scroll_behavior.dart';
import 'package:typie/screens/native_editor/controller/ticker_loop.dart';
import 'package:typie/screens/native_editor/editor_input_view.dart';
import 'package:typie/screens/native_editor/external/models.dart';
import 'package:typie/screens/native_editor/handler/keyboard_handler.dart';
import 'package:typie/screens/native_editor/state/editor_state.dart';
import 'package:typie/screens/native_editor/toolbar/floating/floating.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/toolbar/toolbar.dart';
import 'package:typie/screens/native_editor/upload_manager.dart';
import 'package:typie/screens/native_editor/view/page_list.dart';
import 'package:typie/screens/native_editor/view/scrollbar/editor_scrollbar.dart';
import 'package:typie/services/keyboard.dart';
import 'package:typie/services/preference.dart';

class EditorView extends HookWidget {
  const EditorView({
    required this.controller,
    required this.width,
    required this.height,
    required this.title,
    required this.subtitle,
    required this.onTitleChanged,
    required this.onSubtitleChanged,
    required this.titleFocusNode,
    required this.subtitleFocusNode,
    super.key,
  });

  final EditorController controller;
  final double width;
  final double height;
  final String title;
  final String subtitle;
  final ValueChanged<String> onTitleChanged;
  final ValueChanged<String> onSubtitleChanged;
  final FocusNode titleFocusNode;
  final FocusNode subtitleFocusNode;

  @override
  Widget build(BuildContext context) {
    final editor = controller.editor;

    final tickerProvider = useSingleTickerProvider();
    final scrollController = useScrollController();
    final horizontalScrollController = useScrollController();
    final inputKey = useMemoized(GlobalKey<EditorInputViewState>.new);
    final isComposing = useRef(false);
    final isLongPressing = useRef(false);
    final deleteStartTime = useRef<DateTime?>(null);
    final lastDeleteSignal = useRef<DateTime?>(null);

    final keyboardHeight = useValueNotifier<double>(0);
    final isKeyboardVisible = useValueNotifier<bool>(false);
    final keyboardType = useValueNotifier<KeyboardType>(KeyboardType.software);
    final isEditorFocused = useValueNotifier<bool>(false);
    final bottomToolbarMode = useValueNotifier<BottomToolbarMode>(BottomToolbarMode.hidden);
    final secondaryToolbarMode = useValueNotifier<SecondaryToolbarMode>(SecondaryToolbarMode.hidden);

    final uniformMarks = useValueNotifier<List<Map<String, dynamic>>>([]);
    final mixedMarks = useValueNotifier<List<String>>([]);
    final selectionStats = useValueNotifier<Map<String, dynamic>>({});

    final externalElements = useValueNotifier<List<ExternalElement>>([]);
    final uploadManager = useMemoized(UploadManager.new);
    final suppressScrollbarShow = useValueNotifier(false);
    final suppressScrollbarTimer = useRef<Timer?>(null);
    final titleHeaderHeight = useRef<double>(0);

    useEffect(() => uploadManager.dispose, []);

    final focusController = useMemoized(
      () => EditorFocusController(
        inputKey: inputKey,
        onFocusChanged: controller.setFocused,
        onCommitComposing: () {
          if (isComposing.value) {
            isComposing.value = false;
            controller.dispatch({'type': 'commitPreedit'});
          }
          inputKey.currentState?.resetInputContext();
        },
      ),
      [controller],
    );

    useEffect(() {
      controller.setClearFocusCallback(focusController.clearFocus);
      return null;
    }, [focusController]);

    useEffect(() {
      void scrollToTop() {
        if (scrollController.hasClients) {
          suppressScrollbarTimer.value?.cancel();
          suppressScrollbarShow.value = true;
          unawaited(scrollController.animateTo(0, duration: const Duration(milliseconds: 100), curve: Curves.easeOut));
          suppressScrollbarTimer.value = Timer(const Duration(milliseconds: 150), () {
            suppressScrollbarShow.value = false;
          });
        }
      }

      void onTitleFocusChange() {
        if (titleFocusNode.hasFocus) {
          if (focusController.isActive) {
            focusController.clearFocus();
          }
          scrollToTop();
        }
      }

      void onSubtitleFocusChange() {
        if (subtitleFocusNode.hasFocus) {
          if (focusController.isActive) {
            focusController.clearFocus();
          }
          scrollToTop();
        }
      }

      titleFocusNode.addListener(onTitleFocusChange);
      subtitleFocusNode.addListener(onSubtitleFocusChange);
      return () {
        titleFocusNode.removeListener(onTitleFocusChange);
        subtitleFocusNode.removeListener(onSubtitleFocusChange);
      };
    }, [focusController]);

    final keyboardHandler = useMemoized(
      () => EditorKeyboardHandler(dispatch: controller.dispatch, commitComposing: focusController.onCommitComposing),
      [controller, focusController],
    );

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
    final pref = useService<Pref>();

    useEffect(() {
      final subscription = keyboard.onHeightChange.listen((double height) {
        final wasVisible = isKeyboardVisible.value;
        if (height > 0) {
          keyboardHeight.value = height;
          bottomToolbarMode.value = BottomToolbarMode.hidden;
        }
        isKeyboardVisible.value = height > 0;

        if (!wasVisible && height > 0) {
          WidgetsBinding.instance.addPostFrameCallback((_) {
            if (!scrollController.hasClients) {
              return;
            }
            final layout = controller.state.layout;
            final cursor = controller.state.cursor;
            if (layout == null ||
                cursor == null ||
                !cursor.show ||
                isLongPressing.value ||
                !controller.state.isFocused) {
              return;
            }
            final horizontalPadding = layout.isPaginated ? 40.0 : 0.0;
            EditorScrollBehavior(
              scrollController: scrollController,
              horizontalScrollController: horizontalScrollController,
              horizontalPadding: horizontalPadding,
              titleHeaderHeight: titleHeaderHeight.value,
              typewriterEnabled: pref.typewriterEnabled,
              typewriterPosition: pref.typewriterPosition,
            ).scrollToCursor(cursor, layout);
          });
        }
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
        if (titleFocusNode.hasFocus || subtitleFocusNode.hasFocus) {
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

    useEffect(
      () {
        uniformMarks.value = state.state.uniformMarks;
        mixedMarks.value = state.state.mixedMarks;
        selectionStats.value = state.state.selectionStats;
        externalElements.value = state.state.externalElements;
        isEditorFocused.value = state.state.isFocused;
        return null;
      },
      [
        state.state.uniformMarks,
        state.state.mixedMarks,
        state.state.selectionStats,
        state.state.externalElements,
        state.state.isFocused,
      ],
    );

    final pendingScroll = useRef<VoidCallback?>(null);
    final lastScrollRenderVersion = useRef<Object?>(state.state.renderVersion);

    useEffect(() {
      if (cursor != null && cursor.show) {
        focusController.updateCursor(cursor.x, cursor.y, cursor.height);

        if (currentLayout != null && !isLongPressing.value && state.state.isFocused) {
          final horizontalPadding = currentLayout.isPaginated ? 40.0 : 0.0;
          if (lastScrollRenderVersion.value != state.state.renderVersion) {
            lastScrollRenderVersion.value = state.state.renderVersion;
            final capturedCursor = cursor;
            final capturedLayout = currentLayout;
            final useTypewriter = pref.typewriterEnabled && controller.typewriterNeedsScroll;
            if (useTypewriter) {
              controller.typewriterNeedsScroll = false;
            }
            pendingScroll.value = () {
              suppressScrollbarTimer.value?.cancel();
              suppressScrollbarShow.value = true;
              EditorScrollBehavior(
                scrollController: scrollController,
                horizontalScrollController: horizontalScrollController,
                horizontalPadding: horizontalPadding,
                titleHeaderHeight: titleHeaderHeight.value,
                typewriterEnabled: useTypewriter,
                typewriterPosition: pref.typewriterPosition,
              ).scrollToCursor(capturedCursor, capturedLayout);
              suppressScrollbarTimer.value = Timer(const Duration(milliseconds: 150), () {
                suppressScrollbarShow.value = false;
              });
            };
          } else {
            suppressScrollbarTimer.value?.cancel();
            suppressScrollbarShow.value = true;
            EditorScrollBehavior(
              scrollController: scrollController,
              horizontalScrollController: horizontalScrollController,
              horizontalPadding: horizontalPadding,
              titleHeaderHeight: titleHeaderHeight.value,
            ).scrollToCursor(cursor, currentLayout);
            suppressScrollbarTimer.value = Timer(const Duration(milliseconds: 150), () {
              suppressScrollbarShow.value = false;
            });
          }
        }
      }
      return null;
    }, [cursor, state.state.renderVersion]);

    if (currentLayout == null) {
      return const SizedBox.shrink();
    }

    return NativeEditorToolbarScope(
      keyboardHeight: keyboardHeight,
      isKeyboardVisible: isKeyboardVisible,
      keyboardType: keyboardType,
      isEditorFocused: isEditorFocused,
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
      dismissKeyboard: focusController.dismissKeyboard,
      commitComposing: focusController.onCommitComposing,
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
                  lineHighlightEnabled: pref.lineHighlightEnabled,
                  renderVersion: state.state.renderVersion,
                  scrollController: scrollController,
                  horizontalScrollController: horizontalScrollController,
                  onOpenInput: focusController.openInput,
                  onClearFocus: focusController.clearFocus,
                  onCommitComposing: focusController.onCommitComposing,
                  onSelectionStart: () => controller.setSelecting(true),
                  onSelectionEnd: () => controller.setSelecting(false),
                  onLongPressStateChanged: (value) => isLongPressing.value = value,
                  title: title,
                  subtitle: subtitle,
                  onTitleChanged: onTitleChanged,
                  onSubtitleChanged: onSubtitleChanged,
                  titleFocusNode: titleFocusNode,
                  subtitleFocusNode: subtitleFocusNode,
                  onEnterDocument: () {
                    focusController.openInput();
                    controller.dispatch({'type': 'navigate', 'direction': 'documentStart', 'extend': false});
                  },
                  onTitleHeaderHeightChanged: (height) => titleHeaderHeight.value = height,
                  typewriterEnabled: pref.typewriterEnabled,
                  typewriterPosition: pref.typewriterPosition,
                  fromHandle: state.state.fromHandle,
                  toHandle: state.state.toHandle,
                  onRenderComplete: () {
                    final pending = pendingScroll.value;
                    if (pending != null) {
                      pendingScroll.value = null;
                      pending();
                    }
                  },
                ),
                Positioned.fill(
                  child: EditorInputView(
                    key: inputKey,
                    onInsertText: (text) {
                      deleteStartTime.value = null;
                      controller.dispatch({'type': 'input', 'text': text});
                    },
                    onDeleteBackward: () {
                      final now = DateTime.now();
                      final lastSignal = lastDeleteSignal.value;
                      lastDeleteSignal.value = now;

                      final isRepeating = lastSignal != null && now.difference(lastSignal).inMilliseconds < 500;

                      if (!isRepeating) {
                        deleteStartTime.value = null;
                      }

                      deleteStartTime.value ??= now;
                      final duration = now.difference(deleteStartTime.value!).inMilliseconds / 1000.0;

                      if (duration > 3.0) {
                        controller.dispatch({'type': 'deleteSentenceBackward'});
                      } else if (duration > 1.5) {
                        controller.dispatch({'type': 'deleteWordBackward'});
                      } else {
                        controller.dispatch({'type': 'deleteBackward'});
                      }
                    },
                    onSetMarkedText: (text) {
                      isComposing.value = true;
                      controller.dispatch({'type': 'compositionUpdate', 'text': text});
                    },
                    onUnmarkText: () {
                      if (isComposing.value) {
                        isComposing.value = false;
                        controller.dispatch({'type': 'commitPreedit'});
                      }
                    },
                    onCancelMarkedText: () {
                      isComposing.value = false;
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
                    onFocusLost: () {
                      if (bottomToolbarMode.value != BottomToolbarMode.hidden) {
                        return;
                      }
                      focusController.clearFocus();
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
                EditorScrollbar(
                  scrollController: scrollController,
                  horizontalScrollController: horizontalScrollController,
                  layout: currentLayout,
                  viewHeight: height,
                  viewWidth: width,
                  titleHeaderHeight: titleHeaderHeight.value,
                  typewriterEnabled: pref.typewriterEnabled,
                  typewriterPosition: pref.typewriterPosition,
                  cursor: cursor,
                  suppressShowOnScroll: suppressScrollbarShow,
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
