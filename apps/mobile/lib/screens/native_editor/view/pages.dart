import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/screens/native_editor/controller/clipboard.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/view/context_menu.dart';
import 'package:typie/screens/native_editor/view/editor_draggable.dart';
import 'package:typie/screens/native_editor/view/gesture.dart';
import 'package:typie/screens/native_editor/view/page.dart';
import 'package:typie/screens/native_editor/view/scope.dart';
import 'package:typie/screens/native_editor/view/selection.dart';
import 'package:typie/screens/native_editor/view/table_overlay.dart';
import 'package:typie/screens/native_editor/view/title.dart';
import 'package:typie/services/preference.dart';

class PageList extends HookWidget {
  const PageList({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = ContentScope.of(context);
    final pref = useService<Pref>();
    final state = useListenable(scope.controller);

    final layout = state.state.layout!;
    final cursor = state.state.cursor;
    final isFocused = state.state.isFocused;
    final isSelecting = state.state.isSelecting;
    final fromHandle = state.state.selection?.fromBounds;
    final toHandle = state.state.selection?.toBounds;
    final tableOverlays = useValueListenable(scope.controller.tableOverlays);
    final isTableCellSelectorSelection = tableOverlays.any((overlay) => overlay.isFocused && overlay.showCellSelector);

    final verticalScrollController = scope.verticalScrollController;
    final horizontalScrollController = scope.horizontalScrollController;
    final editor = scope.editor;
    final handleMetricsRevision = useValueNotifier(0);

    useValueListenable(scope.titleAreaHeight);

    (int pageIdx, double localY) getPageAtPosition(double y) {
      final geo = scope.geometry;
      final offsets = geo.computeCumulativePageOffsets();
      final scrollOffset = scope.verticalScrollController.hasClients ? scope.verticalScrollController.offset : 0.0;
      final absoluteY = y + scrollOffset;

      if (absoluteY < geo.titleAreaHeight) {
        return (-1, absoluteY);
      }

      final adjustedY = absoluteY - geo.titleAreaHeight;

      var low = 0;
      var high = offsets.length - 1;
      while (low < high) {
        final mid = (low + high) ~/ 2;
        if (offsets[mid] <= adjustedY) {
          low = mid + 1;
        } else {
          high = mid;
        }
      }

      final pageIdx = (low - 1).clamp(0, geo.layout.pages.length - 1);
      final localY = adjustedY - offsets[pageIdx];
      return (pageIdx, localY);
    }

    final showContextMenu = useState(false);
    final wasContextMenuOpen = useRef(false);
    final clipboard = useMemoized(EditorClipboard.new);

    final longPressPosition = scope.longPressPosition;
    final handleDragPosition = scope.handleDragPosition;
    final dropPosition = useValueNotifier<Offset?>(null);

    final gesture = useMemoized(
      () => GestureController(
        verticalScrollController: verticalScrollController,
        horizontalScrollController: horizontalScrollController,
        editor: editor,
        controller: scope.controller,
        getPageAtPosition: getPageAtPosition,
        getPointerX: (localX) {
          final offset = horizontalScrollController.hasClients ? horizontalScrollController.offset : 0.0;
          return localX + offset - scope.geometry.horizontalPadding;
        },
        getHorizontalPadding: () => scope.geometry.horizontalPadding,
      ),
    );

    useEffect(() => gesture.dispose, const []);

    useEffect(() {
      void onScroll() {
        if (showContextMenu.value) {
          showContextMenu.value = false;
        }
      }

      verticalScrollController.addListener(onScroll);
      horizontalScrollController.addListener(onScroll);
      return () {
        verticalScrollController.removeListener(onScroll);
        horizontalScrollController.removeListener(onScroll);
      };
    }, [verticalScrollController, horizontalScrollController]);

    final prevFromHandle = useRef<SelectionHandleInfo?>(null);
    final prevToHandle = useRef<SelectionHandleInfo?>(null);
    final wasSelecting = useRef(false);

    useEffect(() {
      final isCollapsed = fromHandle == null || toHandle == null;
      final handleDragCanceledByTableSelection =
          isTableCellSelectorSelection && !gesture.draggingCellHandle && gesture.draggingHandleType != null;
      final justFinishedSelecting = wasSelecting.value && !isSelecting;
      final selectionChanged = fromHandle != prevFromHandle.value || toHandle != prevToHandle.value;

      final shouldResetTextHandleDrag =
          !gesture.draggingCellHandle && (isCollapsed || handleDragCanceledByTableSelection);

      if (shouldResetTextHandleDrag) {
        gesture
          ..draggingHandleType = null
          ..dragAnchorHandle = null
          ..stopAutoScroll();
        showContextMenu.value = false;
      } else if (!isSelecting) {
        if (justFinishedSelecting) {
          showContextMenu.value = true;
        } else if (selectionChanged) {
          showContextMenu.value = true;
        }
      }

      if (isSelecting) {
        showContextMenu.value = false;
      }

      wasSelecting.value = isSelecting;
      prevFromHandle.value = fromHandle;
      prevToHandle.value = toHandle;
      return null;
    }, [fromHandle, toHandle, isSelecting, isTableCellSelectorSelection]);

    return LayoutBuilder(
      builder: (context, constraints) {
        final viewWidth = constraints.maxWidth;
        final viewHeight = constraints.maxHeight;

        Offset? viewportPositionFromGlobal(Offset globalPosition) {
          final renderBox = context.findRenderObject() as RenderBox?;
          return renderBox?.globalToLocal(globalPosition);
        }

        void endTextHandleDrag() {
          if (gesture.draggingCellHandle) {
            return;
          }
          final hadHandleDrag = gesture.draggingHandleType != null || handleDragPosition.value != null;
          if (!hadHandleDrag) {
            return;
          }
          gesture
            ..draggingHandleType = null
            ..dragAnchorHandle = null
            ..stopAutoScroll();
          handleDragPosition.value = null;
          if (scope.controller.state.isSelecting) {
            scope.controller.setSelecting(false);
          }
        }

        void onHandleDragDown(SelectionHandleType type, DragDownDetails details) {
          final renderBox = context.findRenderObject() as RenderBox?;
          if (renderBox == null) {
            return;
          }
          gesture.pointerDownTouchPosition = renderBox.globalToLocal(details.globalPosition);
        }

        void onHandleDragStart(SelectionHandleType type, DragStartDetails details) {
          gesture.draggingHandleType = type;
          scope.controller.setSelecting(true);

          final renderBox = context.findRenderObject() as RenderBox?;
          if (renderBox == null) {
            return;
          }
          final touchPosition = gesture.pointerDownTouchPosition ?? renderBox.globalToLocal(details.globalPosition);
          final handle = type == SelectionHandleType.from ? fromHandle : toHandle;
          gesture
            ..dragStartTouchPosition = touchPosition
            ..dragStartHandleScreenPosition = gesture.getHandleStemCenter(handle, scope.geometry) ?? touchPosition
            ..dragAnchorHandle = type == SelectionHandleType.from ? toHandle : fromHandle;
        }

        void onHandleDragUpdate(SelectionHandleType type, DragUpdateDetails details) {
          final renderBox = context.findRenderObject() as RenderBox?;
          if (renderBox == null) {
            return;
          }
          final touchPosition = renderBox.globalToLocal(details.globalPosition);
          final startTouch = gesture.dragStartTouchPosition;
          final startHandleScreen = gesture.dragStartHandleScreenPosition;
          final anchorHandle = gesture.dragAnchorHandle;
          if (startTouch == null || startHandleScreen == null || anchorHandle == null) {
            return;
          }

          final delta = touchPosition - startTouch;
          final selectionScreenPosition = startHandleScreen + delta;

          handleDragPosition.value = selectionScreenPosition;

          final (pageIdx, localY) = getPageAtPosition(selectionScreenPosition.dy);
          if (pageIdx >= 0) {
            final pointerX = gesture.getPointerX(selectionScreenPosition.dx);
            editor.dispatch({
              'type': 'extendSelectionTo',
              'anchorPageIdx': anchorHandle.pageIdx,
              'anchorX': anchorHandle.x,
              'anchorY': anchorHandle.y + anchorHandle.height / 2,
              'headPageIdx': pageIdx,
              'headX': pointerX,
              'headY': localY,
            });
          }

          gesture.handleAutoScroll(
            y: touchPosition.dy,
            x: touchPosition.dx,
            viewWidth: viewWidth,
            viewHeight: viewHeight,
            handleDragPosition: handleDragPosition,
            longPressPosition: longPressPosition,
            dropPosition: dropPosition,
          );
        }

        void onHandleDragEnd(SelectionHandleType type, DragEndDetails details) {
          endTextHandleDrag();
        }

        final geo = scope.geometry;
        final offsets = geo.computeCumulativePageOffsets();
        final contentWidth = (layout.pages.firstOrNull?.width ?? 0) + geo.horizontalPadding * 2;
        final needsHorizontalScroll = contentWidth > viewWidth;
        final horizontalPhysics = isSelecting || !needsHorizontalScroll
            ? const NeverScrollableScrollPhysics()
            : const _NonGestureBouncingScrollPhysics();

        final contentBottomPadding = geo.bottomPadding(
          viewportHeight: verticalScrollController.hasClients
              ? verticalScrollController.position.viewportDimension
              : viewHeight,
          cursor: cursor,
          typewriterEnabled: pref.typewriterEnabled,
          typewriterPosition: pref.typewriterPosition,
        );

        final listView = ScrollConfiguration(
          behavior: ScrollConfiguration.of(context).copyWith(scrollbars: false, dragDevices: isSelecting ? {} : null),
          child: SingleChildScrollView(
            controller: verticalScrollController,
            physics: isSelecting ? const NeverScrollableScrollPhysics() : const _NonGestureBouncingScrollPhysics(),
            child: EditorDraggable(
              gesture: gesture,
              child: RawGestureDetector(
                gestures: {
                  ConditionalLongPressGestureRecognizer:
                      GestureRecognizerFactoryWithHandlers<ConditionalLongPressGestureRecognizer>(
                        () => ConditionalLongPressGestureRecognizer(
                          condition: (globalPosition) {
                            if (gesture.draggingCellHandle) {
                              return false;
                            }
                            final renderBox = context.findRenderObject() as RenderBox?;
                            if (renderBox == null) {
                              return true;
                            }
                            final localPosition = renderBox.globalToLocal(globalPosition);
                            final scrollOffset = verticalScrollController.hasClients
                                ? verticalScrollController.offset
                                : 0.0;
                            final viewportY = localPosition.dy - scrollOffset;
                            final (pageIdx, localY) = getPageAtPosition(viewportY);
                            final pointerX = gesture.getPointerX(localPosition.dx);
                            return scope.editor.isSelectionHit(pageIdx, pointerX, localY);
                          },
                          duration: const Duration(milliseconds: 500),
                        ),
                        (ConditionalLongPressGestureRecognizer instance) {
                          instance
                            ..onLongPressStart = (details) {
                              if (gesture.draggingCellHandle) {
                                return;
                              }
                              scope.inputController.commitComposing();
                              final scrollOffset = verticalScrollController.hasClients
                                  ? verticalScrollController.offset
                                  : 0.0;
                              final viewportPosition = Offset(
                                details.localPosition.dx,
                                details.localPosition.dy - scrollOffset,
                              );

                              longPressPosition.value = viewportPosition;
                              scope.isLongPressing.value = true;

                              final draggingHandle = state.state.draggingHandle;
                              final anchorHandle = draggingHandle == SelectionHandleType.from
                                  ? state.state.selection?.toBounds
                                  : state.state.selection?.fromBounds;

                              gesture
                                ..dragStartTouchPosition = details.globalPosition
                                ..dragStartHandleScreenPosition = gesture.getHandleStemCenter(
                                  fromHandle ?? toHandle,
                                  scope.geometry,
                                )
                                ..dragAnchorHandle = anchorHandle
                                ..lastTapTime = null;
                            }
                            ..onLongPressMoveUpdate = (details) {
                              final scrollOffset = verticalScrollController.hasClients
                                  ? verticalScrollController.offset
                                  : 0.0;
                              final viewportPosition = Offset(
                                details.localPosition.dx,
                                details.localPosition.dy - scrollOffset,
                              );

                              final (pageIdx, localY) = getPageAtPosition(viewportPosition.dy);
                              longPressPosition.value = viewportPosition;

                              if (pageIdx >= 0) {
                                final pointerX = gesture.getPointerX(details.localPosition.dx);
                                editor
                                  ..dispatch({
                                    'type': 'pointerDown',
                                    'pageIdx': pageIdx,
                                    'x': pointerX,
                                    'y': localY,
                                    'clickCount': 1,
                                    'button': 'primary',
                                    'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
                                  })
                                  ..dispatch({
                                    'type': 'pointerUp',
                                    'pageIdx': pageIdx,
                                    'x': pointerX,
                                    'y': localY,
                                    'button': 'primary',
                                    'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
                                  });
                                scope.controller.scrollIntoView();
                              }

                              gesture.handleAutoScroll(
                                y: viewportPosition.dy,
                                x: viewportPosition.dx,
                                viewWidth: viewWidth,
                                viewHeight: viewHeight,
                                handleDragPosition: handleDragPosition,
                                longPressPosition: longPressPosition,
                                dropPosition: dropPosition,
                              );
                            }
                            ..onLongPressEnd = (details) {
                              longPressPosition.value = null;
                              gesture.stopAutoScroll();
                              scope.isLongPressing.value = false;
                            };
                        },
                      ),
                },
                child: Column(
                  children: [
                    _MeasuredTitleFields(scope: scope),
                    if (needsHorizontalScroll)
                      SingleChildScrollView(
                        controller: horizontalScrollController,
                        scrollDirection: Axis.horizontal,
                        physics: horizontalPhysics,
                        child: Container(
                          width: (geo.layout.pages.firstOrNull?.width ?? 0) + geo.horizontalPadding * 2,
                          padding: EdgeInsets.only(
                            left: geo.horizontalPadding,
                            right: geo.horizontalPadding,
                            bottom: contentBottomPadding,
                          ),
                          child: Column(
                            children: [
                              for (var i = 0; i < layout.pages.length; i++) ...[
                                _PageSlot(
                                  key: ValueKey(i),
                                  pageIndex: i,
                                  pageTop: geo.titleAreaHeight + offsets[i],
                                  pageBottom: geo.titleAreaHeight + offsets[i] + layout.pages[i].height,
                                ),
                              ],
                            ],
                          ),
                        ),
                      )
                    else
                      Container(
                        width: (geo.layout.pages.firstOrNull?.width ?? 0) + geo.horizontalPadding * 2,
                        padding: EdgeInsets.only(
                          left: geo.horizontalPadding,
                          right: geo.horizontalPadding,
                          bottom: contentBottomPadding,
                        ),
                        child: Column(
                          children: [
                            for (var i = 0; i < layout.pages.length; i++) ...[
                              _PageSlot(
                                key: ValueKey(i),
                                pageIndex: i,
                                pageTop: geo.titleAreaHeight + offsets[i],
                                pageBottom: geo.titleAreaHeight + offsets[i] + layout.pages[i].height,
                              ),
                            ],
                          ],
                        ),
                      ),
                  ],
                ),
              ),
            ),
          ),
        );

        void dispatchTap(Offset localPosition) {
          showContextMenu.value = false;

          final (pageIdx, localY) = getPageAtPosition(localPosition.dy);

          if (pageIdx < 0) {
            return;
          }

          scope.inputController.commitComposing();
          scope.inputController.openInput();

          final now = DateTime.now();
          final prevTime = gesture.lastTapTime;
          final prevPosition = gesture.lastTapPosition;

          var clickCount = 1;
          if (prevTime != null && prevPosition != null) {
            final timeDiff = now.difference(prevTime).inMilliseconds;
            final distance = (localPosition - prevPosition).distance;
            if (timeDiff < 300 && distance < 20) {
              clickCount = 2;
            }
          }

          gesture
            ..lastTapTime = now
            ..lastTapPosition = localPosition;

          final pointerX = gesture.getPointerX(localPosition.dx);

          if (clickCount == 1) {
            final isSelectionHit = scope.editor.isSelectionHit(pageIdx, pointerX, localY);
            if (isSelectionHit) {
              if (!wasContextMenuOpen.value) {
                showContextMenu.value = true;
              }
              return;
            }
          }

          final keysPressed = HardwareKeyboard.instance.logicalKeysPressed;
          final isShiftHeader =
              keysPressed.contains(LogicalKeyboardKey.shiftLeft) || keysPressed.contains(LogicalKeyboardKey.shiftRight);

          final prevCursor = cursor;

          editor
            ..dispatch({
              'type': 'pointerDown',
              'pageIdx': pageIdx,
              'x': pointerX,
              'y': localY,
              'clickCount': clickCount,
              'button': 'primary',
              'modifier': {'shift': isShiftHeader, 'ctrl': false, 'alt': false, 'meta': false},
            })
            ..dispatch({
              'type': 'pointerUp',
              'pageIdx': pageIdx,
              'x': pointerX,
              'y': localY,
              'button': 'primary',
              'modifier': {'shift': isShiftHeader, 'ctrl': false, 'alt': false, 'meta': false},
            });
          scope.controller.scrollIntoView();

          if (clickCount == 1) {
            unawaited(
              scope.controller.waitForNextTick().then((_) {
                if (!context.mounted) {
                  return;
                }

                final newState = scope.controller.state;
                final isCollapsed = newState.selection?.collapsed ?? true;

                final isSameCursor =
                    isCollapsed &&
                    newState.cursor != null &&
                    prevCursor != null &&
                    newState.cursor!.isSamePosition(prevCursor);

                if (isSameCursor) {
                  final isInteractive = scope.editor.isInteractiveHit(pageIdx, pointerX, localY);
                  if (!isInteractive && !wasContextMenuOpen.value) {
                    showContextMenu.value = true;
                  }
                }
              }),
            );
          }
        }

        Widget buildSelectionHandle(SelectionHandleInfo handle, SelectionHandleType type) {
          return SelectionHandle(
            handleInfo: handle,
            type: type,
            onDragDown: onHandleDragDown,
            onDragStart: onHandleDragStart,
            onDragUpdate: onHandleDragUpdate,
            onDragEnd: onHandleDragEnd,
          );
        }

        final gestureDetector = GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTapDown: (details) {
            gesture.tapDispatched = false;
            wasContextMenuOpen.value = showContextMenu.value;
            if (showContextMenu.value) {
              showContextMenu.value = false;
            }

            gesture.tapTimer?.cancel();
            gesture.tapTimer = Timer(const Duration(milliseconds: 150), () {
              gesture.tapDispatched = true;

              final pointerX = gesture.getPointerX(details.localPosition.dx);
              final (pageIdx, localY) = getPageAtPosition(details.localPosition.dy);

              final canDrag = scope.editor.isSelectionHit(pageIdx, pointerX, localY);

              if (!canDrag) {
                dispatchTap(details.localPosition);
              }
            });
          },
          onTapUp: (details) {
            gesture.tapTimer?.cancel();
            gesture.tapTimer = null;
            if (!gesture.tapDispatched) {
              dispatchTap(details.localPosition);
            }
          },
          onTapCancel: () {
            gesture.tapTimer?.cancel();
            gesture.tapTimer = null;
          },

          onPanDown: (details) {
            if (isSelecting) {
              return;
            }
            if (verticalScrollController.hasClients) {
              verticalScrollController.position.hold(() {});
            }
            if (horizontalScrollController.hasClients) {
              horizontalScrollController.position.hold(() {});
            }
          },
          onPanStart: (details) {
            if (verticalScrollController.hasClients) {
              gesture.verticalDrag = verticalScrollController.position.drag(details, () {
                gesture.verticalDrag = null;
              });
            }
            if (needsHorizontalScroll && horizontalScrollController.hasClients) {
              gesture.horizontalDrag = horizontalScrollController.position.drag(details, () {
                gesture.horizontalDrag = null;
              });
            }
          },
          onPanUpdate: (details) {
            gesture.verticalDrag?.update(
              DragUpdateDetails(
                globalPosition: details.globalPosition,
                delta: Offset(0, details.delta.dy),
                primaryDelta: details.delta.dy,
                sourceTimeStamp: details.sourceTimeStamp,
              ),
            );
            gesture.horizontalDrag?.update(
              DragUpdateDetails(
                globalPosition: details.globalPosition,
                delta: Offset(details.delta.dx, 0),
                primaryDelta: details.delta.dx,
                sourceTimeStamp: details.sourceTimeStamp,
              ),
            );
          },
          onPanEnd: (details) {
            gesture.verticalDrag?.end(
              DragEndDetails(
                velocity: Velocity(pixelsPerSecond: Offset(0, details.velocity.pixelsPerSecond.dy)),
                primaryVelocity: details.velocity.pixelsPerSecond.dy,
              ),
            );
            gesture.horizontalDrag?.end(
              DragEndDetails(
                velocity: Velocity(pixelsPerSecond: Offset(details.velocity.pixelsPerSecond.dx, 0)),
                primaryVelocity: details.velocity.pixelsPerSecond.dx,
              ),
            );
            gesture
              ..verticalDrag = null
              ..horizontalDrag = null;
          },
          onPanCancel: () {
            gesture.verticalDrag?.cancel();
            gesture.horizontalDrag?.cancel();
            gesture
              ..verticalDrag = null
              ..horizontalDrag = null;
          },
          child: listView,
        );

        return DropRegion(
          formats: Formats.standardFormats,
          hitTestBehavior: HitTestBehavior.translucent,
          onDropOver: (event) {
            final item = event.session.items.firstOrNull;
            if (item == null) {
              return DropOperation.none;
            }

            final position = event.position.local;
            final (pIdx, localY) = getPageAtPosition(position.dy);

            final pointerX = gesture.getPointerX(position.dx);

            dropPosition.value = position;
            gesture.handleAutoScroll(
              y: position.dy,
              x: position.dx,
              viewWidth: viewWidth,
              viewHeight: viewHeight,
              handleDragPosition: handleDragPosition,
              longPressPosition: longPressPosition,
              dropPosition: dropPosition,
            );

            scope.dndController.handleDragOver(pIdx, pointerX, localY);

            final localData = item.localData;
            if (localData is Map && localData['isInternal'] == true) {
              return DropOperation.move;
            }
            return DropOperation.copy;
          },
          onDropEnter: (event) {
            scope.dndController.handleDragEnter();
          },
          onDropLeave: (event) {
            dropPosition.value = null;
            gesture.stopAutoScroll();
            scope.dndController.handleDragLeave();
          },
          onPerformDrop: (event) async {
            dropPosition.value = null;
            gesture.stopAutoScroll();

            final position = event.position.local;
            final (pageIdx, localY) = getPageAtPosition(position.dy);

            if (pageIdx < 0) {
              scope.dndController.handleDragEnd();
              return;
            }

            final pointerX = gesture.getPointerX(position.dx);

            await scope.dndController.handleDrop(pageIdx: pageIdx, x: pointerX, y: localY, session: event.session);
          },
          child: Listener(
            behavior: HitTestBehavior.translucent,
            onPointerUp: (_) => endTextHandleDrag(),
            onPointerCancel: (_) => endTextHandleDrag(),
            child: Stack(
              clipBehavior: Clip.none,
              children: [
                NotificationListener<ScrollMetricsNotification>(
                  onNotification: (_) {
                    handleMetricsRevision.value++;
                    return false;
                  },
                  child: gestureDetector,
                ),
                TableOverlay(
                  gesture: gesture,
                  viewWidth: viewWidth,
                  viewHeight: viewHeight,
                  dropPosition: dropPosition,
                  globalToViewport: viewportPositionFromGlobal,
                ),
                ListenableBuilder(
                  listenable: Listenable.merge([
                    verticalScrollController,
                    horizontalScrollController,
                    handleMetricsRevision,
                  ]),
                  builder: (context, _) {
                    final fromPos = gesture.getHandlePosition(fromHandle, geo);
                    final toPos = gesture.getHandlePosition(toHandle, geo);

                    return Stack(
                      clipBehavior: Clip.none,
                      children: [
                        if (isFocused && fromHandle != null && fromPos != null)
                          if (!isTableCellSelectorSelection)
                            Positioned(
                              left: fromPos.dx,
                              top: fromPos.dy,
                              child: buildSelectionHandle(fromHandle, SelectionHandleType.from),
                            ),
                        if (isFocused && toHandle != null && toPos != null)
                          if (!isTableCellSelectorSelection)
                            Positioned(
                              left: toPos.dx,
                              top: toPos.dy,
                              child: buildSelectionHandle(toHandle, SelectionHandleType.to),
                            ),
                      ],
                    );
                  },
                ),
                if (showContextMenu.value && longPressPosition.value == null && gesture.draggingHandleType == null)
                  SelectionContextMenu(clipboard: clipboard, onDismiss: () => showContextMenu.value = false),
              ],
            ),
          ),
        );
      },
    );
  }
}

class _PageSlot extends HookWidget {
  const _PageSlot({required this.pageIndex, required this.pageTop, required this.pageBottom, super.key});

  final int pageIndex;
  final double pageTop;
  final double pageBottom;

  @override
  Widget build(BuildContext context) {
    final scope = ContentScope.of(context);
    final verticalScrollController = scope.verticalScrollController;

    bool computeVisibility() {
      if (!verticalScrollController.hasClients) {
        return true;
      }
      final scrollOffset = verticalScrollController.offset;
      final viewHeight = verticalScrollController.position.hasContentDimensions
          ? verticalScrollController.position.viewportDimension
          : 0.0;
      const cacheExtent = 200.0;
      final viewTop = scrollOffset - cacheExtent;
      final viewBottom = scrollOffset + viewHeight + cacheExtent;
      return pageBottom >= viewTop && pageTop <= viewBottom;
    }

    final visible = useState(computeVisibility());

    useEffect(() {
      void updateVisibility() {
        final nowVisible = computeVisibility();
        if (nowVisible != visible.value) {
          visible.value = nowVisible;
        }
      }

      verticalScrollController.addListener(updateVisibility);
      updateVisibility();
      return () => verticalScrollController.removeListener(updateVisibility);
    }, [verticalScrollController, pageTop, pageBottom]);

    final pageHeight = pageBottom - pageTop;

    if (!visible.value) {
      return SizedBox(height: pageHeight + scope.geometry.gapAfterPage(pageIndex));
    }

    return PageItem(pageIndex: pageIndex);
  }
}

class _NonGestureBouncingScrollPhysics extends BouncingScrollPhysics {
  const _NonGestureBouncingScrollPhysics({super.parent});

  @override
  _NonGestureBouncingScrollPhysics applyTo(ScrollPhysics? ancestor) {
    return _NonGestureBouncingScrollPhysics(parent: buildParent(ancestor));
  }

  @override
  bool shouldAcceptUserOffset(ScrollMetrics position) => false;
}

class _MeasuredTitleFields extends HookWidget {
  const _MeasuredTitleFields({required this.scope});

  final ContentScope scope;

  @override
  Widget build(BuildContext context) {
    void measureHeight() {
      final renderBox = context.findRenderObject() as RenderBox?;
      if (renderBox != null && renderBox.hasSize) {
        final height = renderBox.size.height;
        if (scope.titleAreaHeight.value != height) {
          scope.titleAreaHeight.value = height;
        }
      }
    }

    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) => measureHeight());
      return null;
    });

    final title = useValueListenable(scope.title);
    final subtitle = useValueListenable(scope.subtitle);

    return LayoutBuilder(
      builder: (context, constraints) {
        return TitleFields(
          title: title,
          subtitle: subtitle,
          onEnterDocument: () {
            scope.inputController.openInput();
            scope.controller.dispatch({'type': 'navigate', 'direction': 'documentStart', 'extend': false});
            scope.controller.scrollIntoView();
          },
          pageWidth: constraints.maxWidth,
          onFieldTap: scope.inputController.clearFocus,
        );
      },
    );
  }
}
