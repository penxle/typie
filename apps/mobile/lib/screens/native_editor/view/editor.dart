import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/screens/native_editor/controller/input.dart';
import 'package:typie/screens/native_editor/controller/keyboard.dart';
import 'package:typie/screens/native_editor/controller/ticker.dart';
import 'package:typie/screens/native_editor/controller/upload.dart';
import 'package:typie/screens/native_editor/external/models.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/toolbar/floating/floating.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/toolbar/toolbar.dart';
import 'package:typie/screens/native_editor/view/geometry.dart';
import 'package:typie/screens/native_editor/view/input.dart';
import 'package:typie/screens/native_editor/view/pages.dart';
import 'package:typie/screens/native_editor/view/scope.dart';
import 'package:typie/screens/native_editor/view/scroll.dart';
import 'package:typie/screens/native_editor/view/scrollbar.dart';
import 'package:typie/screens/native_editor/view/title.dart';
import 'package:typie/services/keyboard.dart';
import 'package:typie/services/preference.dart';

class EditorView extends HookWidget {
  const EditorView({
    required this.controller,
    required this.title,
    required this.subtitle,
    required this.onTitleChanged,
    required this.onSubtitleChanged,
    required this.titleFocusNode,
    required this.subtitleFocusNode,
    super.key,
  });

  final EditorController controller;
  final String title;
  final String subtitle;
  final ValueChanged<String> onTitleChanged;
  final ValueChanged<String> onSubtitleChanged;
  final FocusNode titleFocusNode;
  final FocusNode subtitleFocusNode;

  @override
  Widget build(BuildContext context) {
    final tickerProvider = useSingleTickerProvider();
    final verticalScrollController = useScrollController();
    final horizontalScrollController = useScrollController();
    final inputKey = useMemoized(GlobalKey<InputViewState>.new);

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
    final titleAreaHeight = useValueNotifier<double>(0);
    final isLongPressing = useValueNotifier(false);
    final pendingScroll = useValueNotifier<VoidCallback?>(null);

    final sizeRef = useRef<(double, double)>((0, 0));

    final titleNotifier = useValueNotifier(title)..value = title;
    final subtitleNotifier = useValueNotifier(subtitle)..value = subtitle;

    useEffect(() => uploadManager.dispose, []);

    final inputController = useMemoized(
      () => InputController(
        inputKey: inputKey,
        dispatch: controller.dispatch,
        onFocusChanged: controller.setFocused,
        getBottomToolbarMode: () => bottomToolbarMode.value,
      ),
      [controller],
    );

    final floatingCursorOrigin = useRef<CursorInfo?>(null);

    useEffect(() {
      controller
        ..setClearFocusCallback(inputController.clearFocus)
        ..setRequestFocusCallback(inputController.requestFocus);

      inputController
        ..floatingCursorBeginHandler = () {
          floatingCursorOrigin.value = controller.state.cursor;
        }
        ..floatingCursorUpdateHandler = (double dx, double dy) {
          final origin = floatingCursorOrigin.value;
          if (origin == null) {
            return;
          }
          final layout = controller.state.layout;
          if (layout == null) {
            return;
          }

          final geo = ContentGeometry(layout: layout, titleAreaHeight: titleAreaHeight.value);

          final newContentX = origin.x + dx;
          final originAbsoluteY = geo.cursorTopInPages(origin);
          final newAbsoluteY = (originAbsoluteY + dy).clamp(0.0, geo.pagesContentHeight);

          final offsets = geo.computeCumulativePageOffsets();
          var low = 0;
          var high = offsets.length - 1;
          while (low < high) {
            final mid = (low + high) ~/ 2;
            if (offsets[mid] <= newAbsoluteY) {
              low = mid + 1;
            } else {
              high = mid;
            }
          }

          final pageIdx = (low - 1).clamp(0, layout.pageCount - 1);
          final localY = newAbsoluteY - offsets[pageIdx];

          final pointerEvent = <String, dynamic>{
            'pageIdx': pageIdx,
            'x': newContentX,
            'y': localY,
            'clickCount': 1,
            'button': 'primary',
            'modifier': <String, bool>{'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
          };

          controller
            ..dispatch({...pointerEvent, 'type': 'pointerDown'})
            ..dispatch({...pointerEvent, 'type': 'pointerUp'});
        }
        ..floatingCursorEndHandler = () {
          floatingCursorOrigin.value = null;
        };

      return () {
        inputController
          ..floatingCursorBeginHandler = null
          ..floatingCursorUpdateHandler = null
          ..floatingCursorEndHandler = null;
      };
    }, [inputController]);

    useEffect(() {
      void scrollToTop() {
        if (verticalScrollController.hasClients) {
          suppressScrollbarTimer.value?.cancel();
          suppressScrollbarShow.value = true;
          unawaited(
            verticalScrollController.animateTo(0, duration: const Duration(milliseconds: 100), curve: Curves.easeOut),
          );
          suppressScrollbarTimer.value = Timer(const Duration(milliseconds: 150), () {
            suppressScrollbarShow.value = false;
          });
        }
      }

      void onTitleFocusChange() {
        if (titleFocusNode.hasFocus) {
          if (inputController.isActive) {
            inputController.clearFocus();
          }
          scrollToTop();
        }
      }

      void onSubtitleFocusChange() {
        if (subtitleFocusNode.hasFocus) {
          if (inputController.isActive) {
            inputController.clearFocus();
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
    }, [inputController]);

    final keyboardHandler = useMemoized(
      () => KeyboardHandler(dispatch: controller.dispatch, commitComposing: inputController.commitComposing),
      [controller, inputController],
    );

    final tickerLoop = useMemoized(
      () => TickerLoop(controller: controller, tickerProvider: tickerProvider, getSize: () => sizeRef.value),
      [controller],
    );

    useEffect(() {
      tickerLoop.start();
      return tickerLoop.dispose;
    }, [tickerLoop]);

    final keyboard = useService<Keyboard>();
    final pref = useService<Pref>();

    void scrollToCursorWith(CursorInfo c, {bool typewriter = false}) {
      scrollToCursor(
        verticalController: verticalScrollController,
        horizontalController: horizontalScrollController,
        geometry: ContentGeometry(titleAreaHeight: titleAreaHeight.value, layout: controller.state.layout!),
        cursor: c,
        typewriterEnabled: typewriter,
        typewriterPosition: typewriter ? pref.typewriterPosition : 0.5,
      );
    }

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
            if (!verticalScrollController.hasClients) {
              return;
            }
            final cursor = controller.state.cursor;
            if (controller.state.layout == null ||
                cursor == null ||
                !cursor.show ||
                isLongPressing.value ||
                !controller.state.isFocused) {
              return;
            }
            scrollToCursorWith(cursor, typewriter: pref.typewriterEnabled);
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
        if (!inputController.isActive) {
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

    final lastScrollRenderVersion = useRef<Object?>(state.state.renderVersion);

    useEffect(() {
      if (cursor == null) {
        return null;
      }

      if (cursor.show) {
        final scrollOffset = verticalScrollController.hasClients ? verticalScrollController.offset : 0.0;
        final screenY = cursor.y - scrollOffset;
        inputController.updateCursor(cursor.x, screenY, cursor.height, cursor.precedingCharWidths);
      }

      final shouldScroll =
          (cursor.show || cursor.scrollToCursor) &&
          currentLayout != null &&
          !isLongPressing.value &&
          state.state.isFocused;

      if (shouldScroll) {
        if (lastScrollRenderVersion.value != state.state.renderVersion) {
          lastScrollRenderVersion.value = state.state.renderVersion;
          final capturedCursor = cursor;
          final useTypewriter = cursor.show && pref.typewriterEnabled && controller.typewriterNeedsScroll;
          if (useTypewriter) {
            controller.typewriterNeedsScroll = false;
          }
          pendingScroll.value = () {
            suppressScrollbarTimer.value?.cancel();
            suppressScrollbarShow.value = true;
            scrollToCursorWith(capturedCursor, typewriter: useTypewriter);
            suppressScrollbarTimer.value = Timer(const Duration(milliseconds: 150), () {
              suppressScrollbarShow.value = false;
            });
          };
        } else if (cursor.show) {
          suppressScrollbarTimer.value?.cancel();
          suppressScrollbarShow.value = true;
          scrollToCursorWith(cursor);
          suppressScrollbarTimer.value = Timer(const Duration(milliseconds: 150), () {
            suppressScrollbarShow.value = false;
          });
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
      requestFocus: inputController.requestFocus,
      clearFocus: inputController.clearFocus,
      dismissKeyboard: inputController.dismissKeyboard,
      commitComposing: inputController.commitComposing,
      child: ContentScope(
        controller: controller,
        verticalScrollController: verticalScrollController,
        horizontalScrollController: horizontalScrollController,
        inputController: inputController,
        isLongPressing: isLongPressing,
        titleAreaHeight: titleAreaHeight,
        title: titleNotifier,
        subtitle: subtitleNotifier,
        onTitleChanged: onTitleChanged,
        onSubtitleChanged: onSubtitleChanged,
        titleFocusNode: titleFocusNode,
        subtitleFocusNode: subtitleFocusNode,
        pendingScroll: pendingScroll,
        child: LayoutBuilder(
          builder: (context, constraints) {
            sizeRef.value = (constraints.maxWidth, constraints.maxHeight);
            return Column(
              children: [
                Expanded(
                  child: Stack(
                    children: [
                      const PageList(),
                      const _TitleOverlay(),
                      Positioned(
                        top: titleAreaHeight.value,
                        left: 0,
                        right: 0,
                        bottom: 0,
                        child: InputView(
                          key: inputKey,
                          onInsertText: inputController.onInsertText,
                          onDeleteBackward: inputController.onDeleteBackward,
                          onSetMarkedText: inputController.onSetMarkedText,
                          onUnmarkText: inputController.onUnmarkText,
                          onCancelMarkedText: inputController.onCancelMarkedText,
                          onPerformAction: inputController.onPerformAction,
                          onShortcut: inputController.onShortcut,
                          onFloatingCursorBegin: inputController.onFloatingCursorBegin,
                          onFloatingCursorUpdate: inputController.onFloatingCursorUpdate,
                          onFloatingCursorEnd: inputController.onFloatingCursorEnd,
                          onFocusLost: inputController.onFocusLost,
                          onReady: inputController.onInputReady,
                          onReplaceBackward: inputController.onReplaceBackward,
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
                        viewHeight: constraints.maxHeight,
                        viewWidth: constraints.maxWidth,
                        suppressShowOnScroll: suppressScrollbarShow,
                      ),
                    ],
                  ),
                ),
                const NativeEditorToolbar(),
              ],
            );
          },
        ),
      ),
    );
  }
}

class _TitleOverlay extends HookWidget {
  const _TitleOverlay();

  @override
  Widget build(BuildContext context) {
    final scope = ContentScope.of(context);
    final titleFieldsKey = useMemoized(GlobalKey.new);

    return Positioned(
      top: 0,
      left: 0,
      right: 0,
      child: AnimatedBuilder(
        animation: scope.verticalScrollController,
        builder: (context, child) {
          final offset = scope.verticalScrollController.hasClients ? scope.verticalScrollController.offset : 0.0;
          return Transform.translate(offset: Offset(0, -offset), child: child);
        },
        child: _MeasuredTitleFields(key: titleFieldsKey, scope: scope),
      ),
    );
  }
}

class _MeasuredTitleFields extends StatefulWidget {
  const _MeasuredTitleFields({required this.scope, super.key});

  final ContentScope scope;

  @override
  State<_MeasuredTitleFields> createState() => _MeasuredTitleFieldsState();
}

class _MeasuredTitleFieldsState extends State<_MeasuredTitleFields> {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _measureHeight());
  }

  @override
  void didUpdateWidget(_MeasuredTitleFields oldWidget) {
    super.didUpdateWidget(oldWidget);
    WidgetsBinding.instance.addPostFrameCallback((_) => _measureHeight());
  }

  void _measureHeight() {
    final renderBox = context.findRenderObject() as RenderBox?;
    if (renderBox != null && renderBox.hasSize) {
      final height = renderBox.size.height;
      if (widget.scope.titleAreaHeight.value != height) {
        widget.scope.titleAreaHeight.value = height;
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final scope = widget.scope;
    return ValueListenableBuilder<String>(
      valueListenable: scope.title,
      builder: (context, title, _) {
        return ValueListenableBuilder<String>(
          valueListenable: scope.subtitle,
          builder: (context, subtitle, _) {
            return LayoutBuilder(
              builder: (context, constraints) {
                return TitleFields(
                  title: title,
                  subtitle: subtitle,
                  onEnterDocument: () {
                    scope.inputController.openInput();
                    scope.controller.dispatch({'type': 'navigate', 'direction': 'documentStart', 'extend': false});
                  },
                  pageWidth: constraints.maxWidth,
                  onFieldTap: scope.inputController.clearFocus,
                );
              },
            );
          },
        );
      },
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
