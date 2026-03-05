import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
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
import 'package:typie/screens/native_editor/context.dart';
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
import 'package:typie/screens/native_editor/view/interaction/state.dart';
import 'package:typie/screens/native_editor/view/magnifier.dart';
import 'package:typie/screens/native_editor/view/pages.dart';
import 'package:typie/screens/native_editor/view/repaste_as_text.dart';
import 'package:typie/screens/native_editor/view/scope.dart';
import 'package:typie/screens/native_editor/view/scroll.dart';
import 'package:typie/screens/native_editor/view/scrollbar.dart';
import 'package:typie/screens/native_editor/view/zoom.dart';
import 'package:typie/screens/native_editor/view/zoom_overlay.dart';
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
    final inputKey = useMemoized(GlobalKey<EditorTextInputState>.new);
    EditorScope.of(context).showInputRecordingSheet = () {
      inputKey.currentState?.showRecordingSheet();
    };
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
    final longPressPosition = useValueNotifier<Offset?>(null);
    final handleDragPosition = useValueNotifier<Offset?>(null);
    final interactionState = useMemoized(EditorInteractionState.new);
    final pendingScroll = useValueNotifier<VoidCallback?>(null);
    final pendingScrollPageIdx = useValueNotifier<int?>(null);
    final presentedViewport = useValueNotifier<PresentedViewport>(
      PresentedViewport.base(
        cursor: controller.state.cursor,
        renderVersion: controller.state.cursor == null ? null : controller.state.renderVersion,
      ),
    );
    final zoomViewportWidth = useValueNotifier<double>(0);
    final displayZoom = useValueNotifier<double>(1);
    final renderZoom = useValueNotifier<double>(1);
    final renderZoomTimer = useRef<Timer?>(null);
    final currentZoomViewportWidth = useValueListenable(zoomViewportWidth);
    final currentDisplayZoom = useValueListenable(displayZoom);
    final currentRenderZoom = useValueListenable(renderZoom);
    final sheetBottomInset = useValueListenable(controller.sheetBottomInset);
    final cursorFollowScrollActive = useRef(false);
    final cursorFollowScrollMode = useRef(ScrollMode.auto);
    final typewriterRecoveryPending = useRef(false);
    final previousExternalElements = useRef<List<ExternalElement>?>(controller.state.externalElements);

    final lastSize = useRef<(double, double, double)>((0, 0, 0));

    final titleNotifier = useValueNotifier(title)..value = title;
    final subtitleNotifier = useValueNotifier(subtitle)..value = subtitle;

    final state = useListenable(controller);
    final currentLayout = state.state.layout;
    final isPaginatedLayout = currentLayout is PaginatedLayout;
    final renderedCursorValue = useValueListenable(presentedViewport).cursor;
    final didApplyPaginatedInitialZoom = useRef(false);

    useEffect(() {
      return () => renderZoomTimer.value?.cancel();
    }, []);

    useEffect(() {
      return () {
        suppressScrollbarTimer.value?.cancel();
        suppressScrollbarTimer.value = null;
      };
    }, []);

    void setSuppressScrollbarVisibility(bool visible) {
      if (!context.mounted) {
        return;
      }
      suppressScrollbarShow.value = visible;
    }

    void setZoom(double zoom, {bool commitRender = false}) {
      final layout = controller.state.layout;
      final next = switch (layout) {
        PaginatedLayout(:final pageWidth) => clampPaginatedZoom(
          zoom: zoom,
          pageWidth: pageWidth,
          viewportWidth: zoomViewportWidth.value > 0 ? zoomViewportWidth.value : pageWidth,
        ),
        _ => 1.0,
      };
      final nextRender = renderZoomForDisplay(next);

      if (zoomDiffers(displayZoom.value, next)) {
        displayZoom.value = next;
      }

      renderZoomTimer.value?.cancel();

      if (commitRender) {
        if (zoomDiffers(renderZoom.value, nextRender)) {
          renderZoom.value = nextRender;
        }
        return;
      }

      renderZoomTimer.value = Timer(renderZoomDebounce, () {
        final latestLayout = controller.state.layout;
        final latestIsPaginated = latestLayout is PaginatedLayout;
        final latestDisplay = latestIsPaginated ? displayZoom.value : 1.0;
        final latestRender = renderZoomForDisplay(latestDisplay);
        if (zoomDiffers(renderZoom.value, latestRender)) {
          renderZoom.value = latestRender;
        }
      });
    }

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
            file: (f) => uploadManager.addFileAsset(
              f.id,
              FileAsset(id: f.id, url: f.url, name: f.name, size: int.parse(f.size.value)),
            ),
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
        getEditorSelection: () => state.state.selection,
      ),
      [controller],
    );

    final dndController = useMemoized(() => DndController(editor: controller.editor, controller: controller), [
      controller,
    ]);

    useEffect(() => interactionState.dispose, [interactionState]);
    useEffect(() => dndController.dispose, [dndController]);

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
          final zoom = displayZoom.value;

          final geo = ContentGeometry(
            layout: layout,
            pages: controller.state.pages,
            titleAreaHeight: titleAreaHeight.value,
            zoom: zoom,
          );

          final newContentX = origin.x + geo.toLogicalX(dx);
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
          final localY = geo.toLogicalY(newAbsoluteY - offsets[pageIdx]);

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
        final verticalPosition = resolveScrollPosition(verticalScrollController);
        if (verticalPosition == null || !verticalPosition.hasContentDimensions) {
          return;
        }
        suppressScrollbarTimer.value?.cancel();
        setSuppressScrollbarVisibility(true);
        unawaited(verticalPosition.animateTo(0, duration: const Duration(milliseconds: 100), curve: Curves.easeOut));
        suppressScrollbarTimer.value = Timer(const Duration(milliseconds: 150), () {
          suppressScrollbarTimer.value = null;
          setSuppressScrollbarVisibility(false);
        });
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
        reconcileInput: inputController.invalidate,
        scrollIntoView: controller.scrollIntoView,
        onShortcut: inputController.onShortcut,
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

    bool scrollToCursorWith(CursorInfo c, {bool typewriter = false}) {
      return scrollToCursor(
        verticalController: verticalScrollController,
        horizontalController: horizontalScrollController,
        geometry: ContentGeometry(
          titleAreaHeight: titleAreaHeight.value,
          layout: controller.state.layout!,
          pages: controller.state.pages,
          selection: controller.state.selection,
          zoom: currentDisplayZoom,
        ),
        cursor: c,
        typewriterEnabled: typewriter,
        typewriterPosition: typewriter ? pref.typewriterPosition : 0.5,
      );
    }

    void registerCursorAutoScroll({required bool typewriter}) {
      cursorFollowScrollActive.value = true;
      cursorFollowScrollMode.value = typewriter ? ScrollMode.typewriter : ScrollMode.auto;
    }

    bool canApplyCursorScrollNow() {
      final verticalPosition = resolveScrollPosition(verticalScrollController);
      return verticalPosition != null && verticalPosition.hasContentDimensions;
    }

    void runCursorScroll(CursorInfo targetCursor, {required bool typewriter}) {
      suppressScrollbarTimer.value?.cancel();
      setSuppressScrollbarVisibility(true);
      final didScroll = scrollToCursorWith(targetCursor, typewriter: typewriter);
      if (didScroll) {
        registerCursorAutoScroll(typewriter: typewriter);
      }
      suppressScrollbarTimer.value = Timer(const Duration(milliseconds: 150), () {
        suppressScrollbarTimer.value = null;
        setSuppressScrollbarVisibility(false);
        if (presentedViewport.value.hasProjectedMetrics) {
          presentedViewport.value = presentedViewport.value.clearProjection();
        }
      });
    }

    void syncInputCursorPosition([CursorInfo? nextCursor]) {
      if (!context.mounted) {
        return;
      }
      final activeCursor = nextCursor ?? presentedViewport.value.cursor;
      final layout = controller.state.layout;
      if (layout == null || activeCursor == null || !activeCursor.visible) {
        return;
      }

      final verticalScrollOffset = resolveScrollOffset(verticalScrollController);
      final zoom = layout is PaginatedLayout ? currentDisplayZoom : 1.0;
      final geo = ContentGeometry(
        layout: layout,
        pages: controller.state.pages,
        titleAreaHeight: titleAreaHeight.value,
        zoom: zoom,
      );
      final horizontalMetrics = resolveHorizontalScrollMetrics(
        controller: horizontalScrollController,
        contentWidth: geo.contentWidth,
        fallbackViewportDimension: MediaQuery.sizeOf(context).width,
      );
      final viewportWidth = horizontalMetrics.viewportDimension;
      final horizontalScrollOffset = horizontalMetrics.scrollOffset;
      final screenY = geo.titleAreaHeight + geo.cursorTopInPages(activeCursor) - verticalScrollOffset;
      final screenX =
          geo.contentStartX(viewportWidth: viewportWidth, horizontalScrollOffset: horizontalScrollOffset) +
          geo.toDisplayX(activeCursor.x);
      inputController.updateCursor(screenX, screenY, geo.toDisplayY(activeCursor.height));
    }

    PresentedViewport buildPresentedViewportSnapshot({required CursorInfo? cursor, required bool projectTypewriter}) {
      final renderVersion = cursor == null ? null : controller.state.renderVersion;
      if (!projectTypewriter || cursor == null) {
        return PresentedViewport.base(cursor: cursor, renderVersion: renderVersion);
      }

      final layout = controller.state.layout;
      final verticalPosition = resolveScrollPosition(verticalScrollController);
      if (layout == null || !cursor.visible || verticalPosition == null || !verticalPosition.hasContentDimensions) {
        return PresentedViewport.base(cursor: cursor, renderVersion: renderVersion);
      }

      final zoom = layout is PaginatedLayout ? currentDisplayZoom : 1.0;
      final geo = ContentGeometry(
        titleAreaHeight: titleAreaHeight.value,
        layout: layout,
        pages: controller.state.pages,
        selection: controller.state.selection,
        zoom: zoom,
      );
      final viewportHeight = verticalPosition.viewportDimension;
      final cursorHeight = geo.toDisplayY(cursor.height);
      final availableRange = viewportHeight - cursorHeight;
      final targetScroll = geo.cursorTopInContent(cursor) - availableRange * pref.typewriterPosition;
      final totalContentHeight = geo.totalContentHeight(
        viewportHeight: viewportHeight,
        cursor: cursor,
        typewriterEnabled: true,
        typewriterPosition: pref.typewriterPosition,
      );
      final maxScrollExtent = (totalContentHeight - viewportHeight).clamp(0.0, double.infinity);
      final scrollOffset = targetScroll.clamp(0.0, maxScrollExtent);

      return PresentedViewport.projected(
        cursor: cursor,
        renderVersion: renderVersion,
        projectedScrollOffset: scrollOffset,
        projectedMaxScrollExtent: maxScrollExtent,
        projectedViewportHeight: viewportHeight,
      );
    }

    void setRenderedCursorSnapshot(CursorInfo? nextCursor, {bool projectTypewriter = false}) {
      presentedViewport.value = buildPresentedViewportSnapshot(
        cursor: nextCursor,
        projectTypewriter: projectTypewriter,
      );
    }

    void queueRenderSynchronizedCursorUpdate({
      required CursorInfo nextCursor,
      required bool typewriter,
      required int? targetPageIdx,
    }) {
      pendingScrollPageIdx.value = targetPageIdx;
      pendingScroll.value = () {
        pendingScrollPageIdx.value = null;
        setRenderedCursorSnapshot(nextCursor, projectTypewriter: typewriter);
        runCursorScroll(nextCursor, typewriter: typewriter);
        syncInputCursorPosition(nextCursor);
      };
    }

    void applyCursorScrollAndVisual(
      CursorInfo nextCursor, {
      required bool typewriter,
      bool synchronizeWithRender = true,
    }) {
      final shouldSynchronizeWithRender =
          synchronizeWithRender &&
          typewriter &&
          !identical(presentedViewport.value.renderVersion, controller.state.renderVersion);
      if (shouldSynchronizeWithRender) {
        queueRenderSynchronizedCursorUpdate(
          nextCursor: nextCursor,
          typewriter: true,
          targetPageIdx: nextCursor.pageIdx,
        );
        return;
      }

      if (canApplyCursorScrollNow()) {
        pendingScroll.value = null;
        pendingScrollPageIdx.value = null;
        setRenderedCursorSnapshot(nextCursor, projectTypewriter: typewriter);
        runCursorScroll(nextCursor, typewriter: typewriter);
        syncInputCursorPosition(nextCursor);
      } else {
        queueRenderSynchronizedCursorUpdate(nextCursor: nextCursor, typewriter: typewriter, targetPageIdx: null);
      }
    }

    useEffect(() {
      final subscription = keyboard.onHeightChange.listen((double height) {
        final nextHeight = height < 0 ? 0.0 : height;
        final wasVisible = isKeyboardVisible.value;
        keyboardHeight.value = nextHeight;
        if (nextHeight > 0) {
          if (!wasVisible) {
            bottomToolbarMode.value = BottomToolbarMode.hidden;
          }
        }
        isKeyboardVisible.value = nextHeight > 0;

        if (!wasVisible && nextHeight > 0) {
          WidgetsBinding.instance.addPostFrameCallback((_) {
            final verticalPosition = resolveScrollPosition(verticalScrollController);
            if (verticalPosition == null || !verticalPosition.hasContentDimensions) {
              return;
            }
            final cursor = controller.state.cursor;
            final interaction = interactionState.snapshot();
            if (controller.state.layout == null ||
                cursor == null ||
                !cursor.visible ||
                interaction.isLongPressing ||
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

    useEffect(() {
      void applyPendingCursorScroll() {
        final pendingMode = controller.pendingScrollMode;
        final nextLayout = controller.state.layout;
        final nextCursor = controller.state.cursor;
        final nextExternalElements = controller.state.externalElements;
        final externalElementsChanged = !identical(previousExternalElements.value, nextExternalElements);
        previousExternalElements.value = nextExternalElements;

        if (nextCursor == null) {
          typewriterRecoveryPending.value = false;
          cursorFollowScrollActive.value = false;
          if (pendingMode != null && !controller.pendingScrollWaitForCursorUpdate) {
            controller.clearPendingScroll();
          }
          pendingScroll.value = null;
          pendingScrollPageIdx.value = null;
          setRenderedCursorSnapshot(null);
          return;
        }

        final focused = controller.state.isFocused;
        final interaction = interactionState.snapshot();
        final blockedByInteraction = interaction.isLongPressing || interaction.isDndActive || !focused;
        if (blockedByInteraction || nextLayout == null || !nextCursor.visible) {
          if (blockedByInteraction) {
            typewriterRecoveryPending.value = false;
          }
          if (!focused || !nextCursor.visible) {
            cursorFollowScrollActive.value = false;
          }
          if (pendingMode != null && !controller.pendingScrollWaitForCursorUpdate) {
            controller.clearPendingScroll();
          }
          pendingScroll.value = null;
          pendingScrollPageIdx.value = null;
          setRenderedCursorSnapshot(nextCursor);
          return;
        }

        if (pendingMode != null) {
          final useTypewriter = pref.typewriterEnabled && pendingMode == ScrollMode.typewriter;
          final waitForCursorUpdate = controller.pendingScrollWaitForCursorUpdate;
          typewriterRecoveryPending.value = useTypewriter;
          controller.clearPendingScroll();
          applyCursorScrollAndVisual(
            nextCursor,
            typewriter: useTypewriter,
            synchronizeWithRender: !waitForCursorUpdate,
          );
          return;
        }

        if (externalElementsChanged) {
          if (cursorFollowScrollActive.value) {
            final followTypewriter = cursorFollowScrollMode.value == ScrollMode.typewriter && pref.typewriterEnabled;
            typewriterRecoveryPending.value = false;
            applyCursorScrollAndVisual(nextCursor, typewriter: followTypewriter);
            return;
          }

          if (typewriterRecoveryPending.value && pref.typewriterEnabled) {
            typewriterRecoveryPending.value = false;
            applyCursorScrollAndVisual(nextCursor, typewriter: true);
            return;
          }

          typewriterRecoveryPending.value = false;
          setRenderedCursorSnapshot(nextCursor);
          return;
        }

        if (pref.typewriterEnabled) {
          applyCursorScrollAndVisual(nextCursor, typewriter: true);
        } else {
          setRenderedCursorSnapshot(nextCursor);
        }
      }

      controller.addListener(applyPendingCursorScroll);
      return () => controller.removeListener(applyPendingCursorScroll);
    }, [controller, dndController, currentDisplayZoom, pref.typewriterEnabled]);

    useEffect(() {
      void onScroll() {
        syncInputCursorPosition();
      }

      verticalScrollController.addListener(onScroll);
      horizontalScrollController.addListener(onScroll);
      return () {
        verticalScrollController.removeListener(onScroll);
        horizontalScrollController.removeListener(onScroll);
      };
    }, [verticalScrollController, horizontalScrollController, currentDisplayZoom]);

    useEffect(() {
      if (!isPaginatedLayout) {
        didApplyPaginatedInitialZoom.value = false;
        setZoom(1, commitRender: true);
        return null;
      }

      if (didApplyPaginatedInitialZoom.value) {
        return null;
      }
      if (currentZoomViewportWidth <= 0) {
        return null;
      }

      final initialZoom = computeInitialPaginatedZoom(
        pageWidth: currentLayout.pageWidth,
        viewportWidth: currentZoomViewportWidth,
      );
      setZoom(initialZoom, commitRender: true);
      didApplyPaginatedInitialZoom.value = true;

      return null;
    }, [isPaginatedLayout, currentLayout, currentZoomViewportWidth]);

    useEffect(() {
      selection.value = state.state.selection;
      attrs.value = state.state.attrs;
      externalElements.value = state.state.externalElements;
      isEditorFocused.value = state.state.isFocused;
      return null;
    }, [state.state.selection, state.state.attrs, state.state.externalElements, state.state.isFocused]);

    useEffect(() {
      inputController.reconcile();
      return null;
    }, [state.state.selection]);

    useEffect(() {
      syncInputCursorPosition(renderedCursorValue);
      return null;
    }, [renderedCursorValue, state.state.renderVersion, currentDisplayZoom]);

    void scrollToOverlay({
      required int pageIdx,
      required double x,
      required double y,
      required double width,
      required double height,
    }) {
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
          zoom: isPaginatedLayout ? currentDisplayZoom : 1.0,
        ),
        pageIdx: pageIdx,
        targetX: x,
        targetY: y,
        targetWidth: width,
        targetHeight: height,
      );
    }

    useEffect(() {
      if (sheetBottomInset <= 0) {
        return null;
      }

      WidgetsBinding.instance.addPostFrameCallback((_) {
        final target = controller.remarkHighlightTarget.value;
        if (target == null) {
          return;
        }
        scrollToOverlay(
          pageIdx: target.pageIdx,
          x: target.boundsX,
          y: target.boundsY,
          width: target.boundsWidth,
          height: target.boundsHeight,
        );
      });

      return null;
    }, [sheetBottomInset]);

    useEffect(() {
      final target = state.state.search.scrollTarget;
      if (target != null) {
        scrollToOverlay(pageIdx: target.pageIdx, x: target.x, y: target.y, width: target.width, height: target.height);
      }
      return null;
    }, [state.state.search.scrollTarget]);

    useEffect(() {
      final target = state.state.spellcheck.scrollTarget;
      final pageIdx = state.state.spellcheck.scrollTargetPageIdx;
      if (target != null && pageIdx != null) {
        scrollToOverlay(pageIdx: pageIdx, x: target.x, y: target.y, width: target.width, height: target.height);
      }
      return null;
    }, [state.state.spellcheck.scrollTarget, state.state.spellcheck.scrollTargetPageIdx]);

    useEffect(() {
      final target = state.state.aiFeedback.scrollTarget;
      final pageIdx = state.state.aiFeedback.scrollTargetPageIdx;
      if (target != null && pageIdx != null) {
        scrollToOverlay(pageIdx: pageIdx, x: target.x, y: target.y, width: target.width, height: target.height);
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
                zoom: currentDisplayZoom,
              ),
              pageIdx: target.pageIdx,
              targetX: target.boundsX,
              targetY: target.boundsY,
              targetWidth: target.boundsWidth,
              targetHeight: target.boundsHeight,
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
      onPointerDown: (_) => inputController.invalidate(),
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
        reconcileInput: inputController.invalidate,
        child: ContentScope(
          controller: controller,
          ticker: ticker,
          dndController: dndController,
          interactionState: interactionState,
          interactionSnapshot: interactionState.listenable,
          verticalScrollController: verticalScrollController,
          horizontalScrollController: horizontalScrollController,
          inputController: inputController,
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
          pendingScrollPageIdx: pendingScrollPageIdx,
          presentedViewport: presentedViewport,
          displayZoom: displayZoom,
          renderZoom: renderZoom,
          setZoom: setZoom,
          child: LayoutBuilder(
            builder: (context, constraints) {
              final width = constraints.maxWidth.floorToDouble();
              final height = constraints.maxHeight;
              if (zoomViewportWidth.value != constraints.maxWidth) {
                final nextViewportWidth = constraints.maxWidth;
                WidgetsBinding.instance.addPostFrameCallback((_) {
                  if (zoomViewportWidth.value != nextViewportWidth) {
                    zoomViewportWidth.value = nextViewportWidth;
                  }
                });
              }
              final zoomForRender = isPaginatedLayout ? currentRenderZoom : 1.0;
              final scaleFactor = MediaQuery.devicePixelRatioOf(context) * zoomForRender;
              final currentSize = (width, height, scaleFactor);
              if (lastSize.value != currentSize) {
                lastSize.value = currentSize;
                controller.dispatch({'type': 'resize', 'width': width, 'height': height, 'scaleFactor': scaleFactor});
              }
              return Column(
                children: [
                  Expanded(
                    child: Stack(
                      fit: StackFit.expand,
                      children: [
                        if (isPaginatedLayout) Positioned.fill(child: ColoredBox(color: context.colors.surfaceSubtle)),
                        NotificationListener<UserScrollNotification>(
                          onNotification: (notification) {
                            if (notification.direction != ScrollDirection.idle) {
                              cursorFollowScrollActive.value = false;
                              typewriterRecoveryPending.value = false;
                            }
                            return false;
                          },
                          child: NotificationListener<ScrollMetricsNotification>(
                            onNotification: (_) {
                              scrollMetricsRevision.value++;
                              return false;
                            },
                            child: const PageList(),
                          ),
                        ),
                        _DocumentPlaceholder(
                          controller: controller,
                          verticalScrollController: verticalScrollController,
                          horizontalScrollController: horizontalScrollController,
                          titleAreaHeight: titleAreaHeight,
                          scrollMetricsRevision: scrollMetricsRevision,
                          documentTemplates: documentTemplates,
                          client: client,
                          displayZoom: displayZoom,
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
                        EditorTextInput(
                          key: inputKey,
                          brightness: context.theme.brightness,
                          controller: inputController,
                        ),
                        if (pref.characterCountFloatingEnabled) const NativeCharacterCountFloating(),
                        const NativeEditorZoomOverlay(),
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
    required this.displayZoom,
  });

  final EditorController controller;
  final ScrollController verticalScrollController;
  final ScrollController horizontalScrollController;
  final ValueNotifier<double> titleAreaHeight;
  final ValueNotifier<int> scrollMetricsRevision;
  final List<GNativeEditorScreen_QueryData_entity_site_documentTemplates> documentTemplates;
  final GraphQLClient client;
  final ValueNotifier<double> displayZoom;

  @override
  Widget build(BuildContext context) {
    if (controller.isDisposed) {
      return const SizedBox.shrink();
    }

    return ListenableBuilder(
      listenable: Listenable.merge([controller, titleAreaHeight, displayZoom]),
      builder: (context, _) {
        final placeholder = controller.state.placeholder;
        if (!placeholder.visible ||
            placeholder.x == null ||
            placeholder.y == null ||
            placeholder.width == null ||
            placeholder.width! <= 0) {
          return const SizedBox.shrink();
        }

        if (titleAreaHeight.value <= 0) {
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
          zoom: displayZoom.value,
        );

        return AnimatedBuilder(
          animation: Listenable.merge([verticalScrollController, horizontalScrollController, scrollMetricsRevision]),
          builder: (context, child) {
            final verticalScroll = resolveScrollOffset(verticalScrollController);
            final horizontalMetrics = resolveHorizontalScrollMetrics(
              controller: horizontalScrollController,
              contentWidth: geo.contentWidth,
              fallbackViewportDimension: MediaQuery.sizeOf(context).width,
            );
            final viewportWidth = horizontalMetrics.viewportDimension;
            final horizontalScroll = horizontalMetrics.scrollOffset;
            final placeholderX = placeholder.x!;
            final placeholderY = placeholder.y!;
            final placeholderWidth = placeholder.width!;

            final top = geo.toDisplayY(placeholderY) + titleAreaHeight.value - verticalScroll;
            final left =
                geo.toDisplayX(placeholderX) +
                geo.contentStartX(viewportWidth: viewportWidth, horizontalScrollOffset: horizontalScroll);
            final zoom = geo.effectiveZoom;

            return Positioned(
              top: top,
              left: left,
              width: placeholderWidth,
              child: Transform.scale(alignment: Alignment.topLeft, scale: zoom, child: child),
            );
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
                    child: TemplateSheet(
                      templates: documentTemplates,
                      editor: controller.editor,
                      controller: controller,
                      client: client,
                    ),
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
