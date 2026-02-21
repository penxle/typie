import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/debounce.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/native_editor/__generated__/native_editor_query.data.gql.dart';
import 'package:typie/screens/native_editor/controller/clipboard.dart';
import 'package:typie/screens/native_editor/controller/dnd_controller.dart';
import 'package:typie/screens/native_editor/controller/input.dart';
import 'package:typie/screens/native_editor/controller/keyboard.dart';
import 'package:typie/screens/native_editor/controller/ticker.dart';
import 'package:typie/screens/native_editor/controller/upload.dart';
import 'package:typie/screens/native_editor/external/models.dart';
import 'package:typie/screens/native_editor/floating/widgets/character_count_floating.dart';
import 'package:typie/screens/native_editor/sheet/template.dart';
import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/screens/native_editor/state/scroll_mode.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/toolbar/floating/floating.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/toolbar/toolbar.dart';
import 'package:typie/screens/native_editor/view/geometry.dart';
import 'package:typie/screens/native_editor/view/input.dart';
import 'package:typie/screens/native_editor/view/magnifier.dart';
import 'package:typie/screens/native_editor/view/pages.dart';
import 'package:typie/screens/native_editor/view/repaste_as_text.dart';
import 'package:typie/screens/native_editor/view/scope.dart';
import 'package:typie/screens/native_editor/view/scroll.dart';
import 'package:typie/screens/native_editor/view/scrollbar.dart';
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
    required this.documentTemplates,
    required this.client,
    this.assets,
    super.key,
  });

  final EditorController controller;
  final String title;
  final String subtitle;
  final ValueChanged<String> onTitleChanged;
  final ValueChanged<String> onSubtitleChanged;
  final FocusNode titleFocusNode;
  final FocusNode subtitleFocusNode;
  final List<GNativeEditorScreen_QueryData_entity_site_documentTemplates> documentTemplates;
  final List<GNativeEditorScreen_QueryData_entity_node__asDocument_assets>? assets;
  final GraphQLClient client;

  @override
  Widget build(BuildContext context) {
    final tickerProvider = useSingleTickerProvider();
    final verticalScrollController = useScrollController();
    final horizontalScrollController = useScrollController();
    final inputKey = useMemoized(GlobalKey<InputViewState>.new);
    final controllerRef = useRef(controller)..value = controller;

    final keyboardHeight = useValueNotifier<double>(0);
    final isKeyboardVisible = useValueNotifier<bool>(false);
    final keyboardType = useValueNotifier<KeyboardType>(KeyboardType.software);
    final isEditorFocused = useValueNotifier<bool>(false);
    final bottomToolbarMode = useValueNotifier<BottomToolbarMode>(BottomToolbarMode.hidden);
    final secondaryToolbarMode = useValueNotifier<SecondaryToolbarMode>(SecondaryToolbarMode.hidden);

    final selection = useValueNotifier<EditorSelection?>(null);
    final attrs = useValueNotifier<List<Map<String, dynamic>>>([]);

    final externalElements = useValueNotifier<List<ExternalElement>>([]);
    final uploadManager = useMemoized(UploadManager.new);
    final suppressScrollbarShow = useValueNotifier(false);
    final suppressScrollbarTimer = useRef<Timer?>(null);
    final titleAreaHeight = useValueNotifier<double>(0);
    final scrollMetricsRevision = useValueNotifier(0);
    useValueListenable(titleAreaHeight);
    final isLongPressing = useValueNotifier(false);
    final longPressPosition = useValueNotifier<Offset?>(null);
    final handleDragPosition = useValueNotifier<Offset?>(null);
    final pendingScroll = useValueNotifier<VoidCallback?>(null);

    final lastSize = useRef<(double, double, double)>((0, 0, 0));

    final titleNotifier = useValueNotifier(title)..value = title;
    final subtitleNotifier = useValueNotifier(subtitle)..value = subtitle;

    useEffect(() => uploadManager.dispose, []);

    useEffect(() {
      if (assets != null) {
        for (final asset in assets!) {
          asset.when(
            image: (img) => uploadManager.addImageAsset(
              img.id,
              ImageAsset(
                id: img.id,
                url: img.url,
                width: img.width,
                height: img.height,
                ratio: img.ratio,
                placeholder: img.placeholder,
              ),
            ),
            file: (f) => uploadManager.addFileAsset(f.id, FileAsset(id: f.id, url: f.url, name: f.name, size: f.size)),
            embed: (e) => uploadManager.addEmbedAsset(
              e.id,
              EmbedAsset(
                id: e.id,
                url: e.url,
                title: e.title,
                description: e.description,
                thumbnailUrl: e.thumbnailUrl,
                html: e.html,
              ),
            ),
            documentArchivedNode: (a) =>
                uploadManager.addArchivedAsset(a.id, ArchivedAsset(id: a.id, content: a.content)),
            orElse: () {},
          );
        }
      }
      return null;
    }, [assets]);

    final inputController = useMemoized(
      () => InputController(
        inputKey: inputKey,
        dispatch: controller.dispatch,
        editor: controller.editor,
        onFocusChanged: controller.setFocused,
        scrollIntoView: controller.scrollIntoView,
        getBottomToolbarMode: () => bottomToolbarMode.value,
        onInputAttempt: () {
          if (bottomToolbarMode.value != BottomToolbarMode.hidden) {
            bottomToolbarMode.value = BottomToolbarMode.hidden;
          }
        },
      ),
      [controller],
    );

    final dndController = useMemoized(() => DndController(editor: controller.editor, controller: controller), [
      controller,
    ]);

    final pref = useService<Pref>();
    final floatingCursorOrigin = useRef<CursorInfo?>(null);

    final characterCountsVersion = useValueListenable(controller.characterCountsVersion);
    final characterCountsDebounce = useDebounce<void>(const Duration(milliseconds: 150));

    useEffect(() {
      characterCountsDebounce.call(controller.refreshCharacterCounts, 'character-counts');
      return null;
    }, [characterCountsVersion, controller]);

    useEffect(() {
      controller
        ..setClearFocusCallback(inputController.clearFocus)
        ..setRequestFocusCallback(inputController.requestFocus);

      inputController
        ..onPasteHandler = () async {
          final payload = await EditorClipboard().getPastePayload();
          if (payload != null) {
            controller
              ..dispatch(payload)
              ..scrollIntoView(mode: ScrollMode.typewriter);
          }
        }
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

          final geo = ContentGeometry(
            layout: layout,
            pages: controller.state.pages,
            titleAreaHeight: titleAreaHeight.value,
          );

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

          final pageIdx = (low - 1).clamp(0, controller.state.pages.length - 1);
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
            ..dispatch({...pointerEvent, 'type': 'pointerUp'})
            ..scrollIntoView();
        }
        ..floatingCursorEndHandler = () {
          floatingCursorOrigin.value = null;
        };

      return () {
        inputController
          ..onPasteHandler = null
          ..floatingCursorBeginHandler = null
          ..floatingCursorUpdateHandler = null
          ..floatingCursorEndHandler = null;
      };
    }, [inputController]);

    useEffect(() {
      void scrollToTop() {
        if (verticalScrollController.hasSingleClient) {
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
      () => KeyboardHandler(
        dispatch: controller.dispatch,
        commitComposing: inputController.commitComposing,
        scrollIntoView: controller.scrollIntoView,
      ),
      [controller, inputController],
    );

    final ticker = useMemoized(
      () => EditorTicker(getController: () => controllerRef.value, tickerProvider: tickerProvider),
      [tickerProvider],
    );

    useEffect(() {
      ticker.start();
      return ticker.dispose;
    }, [ticker]);

    final keyboard = useService<Keyboard>();

    void scrollToCursorWith(CursorInfo c, {bool typewriter = false}) {
      scrollToCursor(
        verticalController: verticalScrollController,
        horizontalController: horizontalScrollController,
        geometry: ContentGeometry(
          titleAreaHeight: titleAreaHeight.value,
          layout: controller.state.layout!,
          pages: controller.state.pages,
          selection: controller.state.selection,
        ),
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
            if (!verticalScrollController.hasSingleClient) {
              return;
            }
            final cursor = controller.state.cursor;
            if (controller.state.layout == null ||
                cursor == null ||
                !cursor.visible ||
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
    final isDropping = useValueListenable(dndController.isDropping);

    useEffect(() {
      selection.value = state.state.selection;
      attrs.value = state.state.attrs;
      externalElements.value = state.state.externalElements;
      isEditorFocused.value = state.state.isFocused;
      return null;
    }, [state.state.selection, state.state.attrs, state.state.externalElements, state.state.isFocused]);

    final lastScrollRenderVersion = useRef<Object?>(state.state.renderVersion);

    useEffect(() {
      if (cursor == null) {
        return null;
      }

      if (cursor.visible) {
        final verticalScrollOffset = verticalScrollController.hasSingleClient ? verticalScrollController.offset : 0.0;
        final horizontalScrollOffset = horizontalScrollController.hasSingleClient
            ? horizontalScrollController.offset
            : 0.0;
        final viewportWidth =
            horizontalScrollController.hasSingleClient && horizontalScrollController.position.hasContentDimensions
            ? horizontalScrollController.position.viewportDimension
            : MediaQuery.sizeOf(context).width;
        final geo = ContentGeometry(
          layout: currentLayout!,
          pages: controller.state.pages,
          titleAreaHeight: titleAreaHeight.value,
        );
        final screenY = geo.titleAreaHeight + geo.cursorTopInPages(cursor) - verticalScrollOffset;
        final screenX =
            geo.contentStartX(viewportWidth: viewportWidth, horizontalScrollOffset: horizontalScrollOffset) + cursor.x;
        inputController.updateCursor(screenX, screenY, cursor.height, cursor.precedingCharWidths);
      }

      final shouldScroll =
          controller.pendingScrollMode != null &&
          currentLayout != null &&
          !isLongPressing.value &&
          !isDropping &&
          state.state.isFocused;

      if (shouldScroll) {
        if (lastScrollRenderVersion.value != state.state.renderVersion) {
          lastScrollRenderVersion.value = state.state.renderVersion;
          final capturedCursor = cursor;
          final useTypewriter =
              cursor.visible && pref.typewriterEnabled && controller.pendingScrollMode == ScrollMode.typewriter;
          if (controller.pendingScrollMode != null) {
            controller.pendingScrollMode = null;
          }
          pendingScroll.value = () {
            suppressScrollbarTimer.value?.cancel();
            suppressScrollbarShow.value = true;
            scrollToCursorWith(capturedCursor, typewriter: useTypewriter);
            suppressScrollbarTimer.value = Timer(const Duration(milliseconds: 150), () {
              suppressScrollbarShow.value = false;
            });
          };
        }
      }
      return null;
    }, [cursor, state.state.renderVersion]);

    void scrollToOverlay({required int pageIdx, required double x, required double y, required double width}) {
      if (currentLayout == null) {
        return;
      }
      scrollToOverlayTarget(
        verticalScrollController: verticalScrollController,
        horizontalScrollController: horizontalScrollController,
        geometry: ContentGeometry(
          titleAreaHeight: titleAreaHeight.value,
          layout: currentLayout,
          pages: controller.state.pages,
        ),
        pageIdx: pageIdx,
        targetX: x,
        targetY: y,
        targetWidth: width,
      );
    }

    useEffect(() {
      final target = state.state.search.scrollTarget;
      if (target != null) {
        scrollToOverlay(pageIdx: target.pageIdx, x: target.x, y: target.y, width: target.width);
      }
      return null;
    }, [state.state.search.scrollTarget]);

    useEffect(() {
      final target = state.state.spellcheck.scrollTarget;
      final pageIdx = state.state.spellcheck.scrollTargetPageIdx;
      if (target != null && pageIdx != null) {
        scrollToOverlay(pageIdx: pageIdx, x: target.x, y: target.y, width: target.width);
      }
      return null;
    }, [state.state.spellcheck.scrollTarget, state.state.spellcheck.scrollTargetPageIdx]);

    useEffect(() {
      final target = state.state.aiFeedback.scrollTarget;
      final pageIdx = state.state.aiFeedback.scrollTargetPageIdx;
      if (target != null && pageIdx != null) {
        scrollToOverlay(pageIdx: pageIdx, x: target.x, y: target.y, width: target.width);
      }
      return null;
    }, [state.state.aiFeedback.scrollTarget, state.state.aiFeedback.scrollTargetPageIdx]);

    useEffect(() {
      void onRemarkScroll() {
        final target = controller.remarkScrollTarget.value;
        if (target != null) {
          final layout = controller.state.layout;
          if (layout != null) {
            scrollToOverlayTarget(
              verticalScrollController: verticalScrollController,
              horizontalScrollController: horizontalScrollController,
              geometry: ContentGeometry(
                titleAreaHeight: titleAreaHeight.value,
                layout: layout,
                pages: controller.state.pages,
              ),
              pageIdx: target.pageIdx,
              targetX: target.boundsX,
              targetY: target.boundsY,
              targetWidth: target.boundsWidth,
            );
          }
          controller.remarkScrollTarget.value = null;
        }
      }

      controller.remarkScrollTarget.addListener(onRemarkScroll);
      return () => controller.remarkScrollTarget.removeListener(onRemarkScroll);
    }, []);

    if (currentLayout == null) {
      return const SizedBox.shrink();
    }

    return Listener(
      onPointerDown: (_) => inputController.commitComposing(),
      child: NativeEditorToolbarScope(
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
          dndController: dndController,
          verticalScrollController: verticalScrollController,
          horizontalScrollController: horizontalScrollController,
          inputController: inputController,
          isLongPressing: isLongPressing,
          longPressPosition: longPressPosition,
          handleDragPosition: handleDragPosition,
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
              final width = constraints.maxWidth.floorToDouble();
              final height = constraints.maxHeight;
              final scaleFactor = MediaQuery.devicePixelRatioOf(context);
              final currentSize = (width, height, scaleFactor);
              if (lastSize.value != currentSize) {
                lastSize.value = currentSize;
                controller.editor.dispatch({
                  'type': 'resize',
                  'width': width,
                  'height': height,
                  'scaleFactor': scaleFactor,
                });
              }
              return Column(
                children: [
                  Expanded(
                    child: Stack(
                      fit: StackFit.expand,
                      children: [
                        NotificationListener<ScrollMetricsNotification>(
                          onNotification: (_) {
                            scrollMetricsRevision.value++;
                            return false;
                          },
                          child: const PageList(),
                        ),
                        _DocumentPlaceholder(
                          controller: controller,
                          verticalScrollController: verticalScrollController,
                          horizontalScrollController: horizontalScrollController,
                          titleAreaHeight: titleAreaHeight,
                          scrollMetricsRevision: scrollMetricsRevision,
                          documentTemplates: documentTemplates,
                          client: client,
                        ),
                        ValueListenableBuilder<Offset?>(
                          valueListenable: longPressPosition,
                          builder: (context, longPress, _) {
                            return ValueListenableBuilder<Offset?>(
                              valueListenable: handleDragPosition,
                              builder: (context, handleDrag, _) {
                                final pos = handleDrag ?? longPress;
                                if (pos == null) {
                                  return const SizedBox.shrink();
                                }
                                return EditorMagnifier(
                                  position: pos,
                                  focalPoint: pos,
                                  pageSize: Size(controller.state.pages.firstOrNull?.width ?? 0, constraints.maxHeight),
                                );
                              },
                            );
                          },
                        ),
                        Positioned.fill(
                          child: IgnorePointer(
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
                              onNavigate: (direction, extend) {
                                inputController.commitComposing();
                                controller
                                  ..dispatch({'type': 'navigate', 'direction': direction, 'extend': extend})
                                  ..scrollIntoView(mode: extend ? ScrollMode.auto : ScrollMode.typewriter);
                              },
                            ),
                          ),
                        ),
                        if (pref.characterCountFloatingEnabled) const NativeCharacterCountFloating(),
                        const Positioned(bottom: 20, right: 20, child: NativeEditorFloatingToolbar()),
                        if (state.state.repasteAsTextEnabled)
                          Positioned(left: 0, right: 0, bottom: 0, child: RepasteAsTextWidget(controller: controller)),
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
      ),
    );
  }
}

class _DocumentPlaceholder extends StatelessWidget {
  const _DocumentPlaceholder({
    required this.controller,
    required this.verticalScrollController,
    required this.horizontalScrollController,
    required this.titleAreaHeight,
    required this.scrollMetricsRevision,
    required this.documentTemplates,
    required this.client,
  });

  final EditorController controller;
  final ScrollController verticalScrollController;
  final ScrollController horizontalScrollController;
  final ValueNotifier<double> titleAreaHeight;
  final ValueNotifier<int> scrollMetricsRevision;
  final List<GNativeEditorScreen_QueryData_entity_site_documentTemplates> documentTemplates;
  final GraphQLClient client;

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: Listenable.merge([controller, titleAreaHeight]),
      builder: (context, _) {
        final placeholder = controller.state.placeholder;
        if (!placeholder.visible ||
            placeholder.x == null ||
            placeholder.y == null ||
            placeholder.width == null ||
            placeholder.width! <= 0) {
          return const SizedBox.shrink();
        }

        final layout = controller.state.layout;
        if (layout == null) {
          return const SizedBox.shrink();
        }

        final geo = ContentGeometry(
          layout: layout,
          pages: controller.state.pages,
          titleAreaHeight: titleAreaHeight.value,
        );

        return AnimatedBuilder(
          animation: Listenable.merge([verticalScrollController, horizontalScrollController, scrollMetricsRevision]),
          builder: (context, child) {
            final verticalScroll = verticalScrollController.hasSingleClient ? verticalScrollController.offset : 0.0;
            final horizontalScroll = horizontalScrollController.hasSingleClient
                ? horizontalScrollController.offset
                : 0.0;
            final viewportWidth =
                horizontalScrollController.hasSingleClient && horizontalScrollController.position.hasContentDimensions
                ? horizontalScrollController.position.viewportDimension
                : MediaQuery.sizeOf(context).width;

            final top = placeholder.y! + titleAreaHeight.value - verticalScroll;
            final left =
                placeholder.x! +
                geo.contentStartX(viewportWidth: viewportWidth, horizontalScrollOffset: horizontalScroll);

            return Positioned(top: top, left: left, width: placeholder.width, child: child!);
          },
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              IgnorePointer(
                child: Text('내용을 입력하거나', style: TextStyle(color: context.colors.textDisabled)),
              ),
              const Gap(4),
              GestureDetector(
                onTap: () async {
                  controller.clearFocus();
                  await context.showBottomSheet(
                    intercept: true,
                    child: TemplateSheet(templates: documentTemplates, editor: controller.editor, client: client),
                  );
                },
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(LucideLightIcons.layout_template, size: 16, color: context.colors.textDisabled),
                    const Gap(4),
                    Text('템플릿 불러오기', style: TextStyle(color: context.colors.textDisabled)),
                  ],
                ),
              ),
            ],
          ),
        );
      },
    );
  }
}
