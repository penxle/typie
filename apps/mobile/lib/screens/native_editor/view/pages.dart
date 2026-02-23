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
import 'package:typie/screens/native_editor/view/geometry.dart';
import 'package:typie/screens/native_editor/view/gesture.dart';
import 'package:typie/screens/native_editor/view/page.dart';
import 'package:typie/screens/native_editor/view/scope.dart';
import 'package:typie/screens/native_editor/view/scroll.dart';
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

    final pages = state.state.pages;
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
      final scrollOffset = scope.verticalScrollController.hasSingleClient ? scope.verticalScrollController.offset : 0.0;
      final absoluteY = y + scrollOffset;
      final extensionAreaTop = (geo.titleAreaHeight - ContentGeometry.pagePadding).clamp(0.0, double.infinity);

      if (absoluteY < extensionAreaTop) {
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

      final pageIdx = (low - 1).clamp(0, geo.pages.length - 1);
      final localY = adjustedY - offsets[pageIdx];
      return (pageIdx, localY);
    }

    final showContextMenu = useState(false);
    final wasContextMenuOpen = useRef(false);
    final clipboard = useMemoized(EditorClipboard.new);
    final viewportWidth = useValueNotifier<double>(0);

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
          final geo = scope.geometry;
          final hScrollOffset = horizontalScrollController.hasSingleClient ? horizontalScrollController.offset : 0.0;
          final viewport = viewportWidth.value;
          return localX - geo.contentStartX(viewportWidth: viewport, horizontalScrollOffset: hScrollOffset);
        },
        getViewportWidth: () => viewportWidth.value,
        isLongPressing: scope.isLongPressing,
      ),
    );
    final gestureState = gesture.state;

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
          isTableCellSelectorSelection && !gesture.isCellHandleDragging && gesture.hasTextHandleDrag;
      final justFinishedSelecting = wasSelecting.value && !isSelecting;
      final selectionChanged = fromHandle != prevFromHandle.value || toHandle != prevToHandle.value;

      final shouldResetTextHandleDrag =
          !gesture.isCellHandleDragging && (isCollapsed || handleDragCanceledByTableSelection);

      if (shouldResetTextHandleDrag) {
        gesture.stopSelectionHandlesAndAutoScroll();
        showContextMenu.value = false;
      } else if (!isSelecting && !gestureState.active) {
        if (justFinishedSelecting) {
          showContextMenu.value = true;
        } else if (selectionChanged) {
          showContextMenu.value = true;
        }
      }

      if (isSelecting || gestureState.active) {
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
        if (viewportWidth.value != viewWidth) {
          viewportWidth.value = viewWidth;
        }

        Offset? viewportPositionFromGlobal(Offset globalPosition) {
          final renderBox = context.findRenderObject() as RenderBox?;
          return renderBox?.globalToLocal(globalPosition);
        }

        bool isConsecutiveTap({required Offset localPosition, required DateTime now}) {
          return gesture.isConsecutiveTap(localPosition: localPosition, now: now);
        }

        void endTextHandleDrag() {
          if (gesture.isCellHandleDragging) {
            return;
          }
          final hadHandleDrag = gesture.hasTextHandleDrag || handleDragPosition.value != null;
          if (!hadHandleDrag) {
            return;
          }
          gesture.stopSelectionHandlesAndAutoScroll();
          handleDragPosition.value = null;
          if (scope.controller.state.isSelecting) {
            scope.controller.setSelecting(false);
          }
        }

        bool dispatchDoubleTapSelection(Offset localPosition) {
          final (pageIdx, localY) = getPageAtPosition(localPosition.dy);
          if (pageIdx < 0) {
            return false;
          }

          showContextMenu.value = false;
          scope.inputController.commitComposing();
          scope.inputController.openInput();

          final pointerX = gesture.getPointerX(localPosition.dx);
          editor
            ..dispatch({
              'type': 'pointerDown',
              'pageIdx': pageIdx,
              'x': pointerX,
              'y': localY,
              'clickCount': 2,
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

          gesture.clearTapHistory();

          scope.controller.scrollIntoView();
          return true;
        }

        void prepareDoubleTapDrag(Offset localPosition) {
          gesture
            ..cancelTapTimer()
            ..setTapDispatched(true)
            ..clearSelectionHandleState()
            ..stopAutoScroll();

          longPressPosition.value = null;
          showContextMenu.value = false;
          gestureState.prepare(localPosition);
          handleDragPosition.value = null;
        }

        void startDoubleTapDrag(Offset localPosition) {
          gesture
            ..cancelTapTimer()
            ..setTapDispatched(true)
            ..setTextHandleDragType(SelectionHandleType.to)
            ..setDragAnchorHandle(null)
            ..stopAutoScroll();

          longPressPosition.value = null;
          showContextMenu.value = false;
          gestureState.begin(localPosition);
          handleDragPosition.value = null;
          scope.controller.setSelecting(true);
        }

        void endDoubleTapDrag() {
          gestureState.stop();
          endTextHandleDrag();
        }

        void updateDoubleTapDragSelection(Offset localPosition) {
          if (!gestureState.dragging) {
            return;
          }

          final startPosition = gestureState.start;
          if (startPosition != null && (localPosition - startPosition).distance < 4) {
            return;
          }

          handleDragPosition.value = localPosition;

          if (gesture.dragAnchorHandle == null) {
            final selection = scope.controller.state.selection;
            final from = selection?.fromBounds;
            final to = selection?.toBounds;
            if (startPosition != null && from != null && to != null) {
              final delta = localPosition - startPosition;
              final towardSelectionEnd = delta.dy > 8 || (delta.dy.abs() <= 8 && delta.dx >= 0);
              gesture
                ..setTextHandleDragType(towardSelectionEnd ? SelectionHandleType.to : SelectionHandleType.from)
                ..setDragAnchorHandle(towardSelectionEnd ? from : to);
            }
          }

          final anchorHandle = gesture.dragAnchorHandle;
          final (pageIdx, localY) = getPageAtPosition(localPosition.dy);
          if (anchorHandle != null && pageIdx >= 0) {
            final pointerX = gesture.getPointerX(localPosition.dx);
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

          if (anchorHandle != null) {
            gesture.handleAutoScroll(
              y: localPosition.dy,
              x: localPosition.dx,
              viewWidth: viewWidth,
              viewHeight: viewHeight,
              handleDragPosition: handleDragPosition,
              longPressPosition: longPressPosition,
              dropPosition: dropPosition,
            );
          }
        }

        void onHandleDragDown(SelectionHandleType type, DragDownDetails details) {
          final renderBox = context.findRenderObject() as RenderBox?;
          if (renderBox == null) {
            return;
          }
          gesture.rememberPointerDown(renderBox.globalToLocal(details.globalPosition));
        }

        void onHandleDragStart(SelectionHandleType type, DragStartDetails details) {
          scope.controller.setSelecting(true);

          final renderBox = context.findRenderObject() as RenderBox?;
          if (renderBox == null) {
            return;
          }
          final touchPosition = gesture.pointerDownTouchPosition() ?? renderBox.globalToLocal(details.globalPosition);
          final handle = type == SelectionHandleType.from ? fromHandle : toHandle;
          gesture.beginTextHandleDrag(
            type: type,
            touchPosition: touchPosition,
            handleScreenPosition: gesture.getHandleStemCenter(handle, scope.geometry) ?? touchPosition,
            anchorHandle: type == SelectionHandleType.from ? toHandle : fromHandle,
          );
        }

        void onHandleDragUpdate(SelectionHandleType type, DragUpdateDetails details) {
          final renderBox = context.findRenderObject() as RenderBox?;
          if (renderBox == null) {
            return;
          }
          final touchPosition = renderBox.globalToLocal(details.globalPosition);
          final dragContext = gesture.selectionHandleDragContext();
          if (dragContext == null) {
            return;
          }

          final delta = touchPosition - dragContext.startTouchPosition;
          final selectionScreenPosition = dragContext.startHandleScreenPosition + delta;

          handleDragPosition.value = selectionScreenPosition;

          final (pageIdx, localY) = getPageAtPosition(selectionScreenPosition.dy);
          if (pageIdx >= 0) {
            final pointerX = gesture.getPointerX(selectionScreenPosition.dx);
            editor.dispatch({
              'type': 'extendSelectionTo',
              'anchorPageIdx': dragContext.anchorHandle.pageIdx,
              'anchorX': dragContext.anchorHandle.x,
              'anchorY': dragContext.anchorHandle.y + dragContext.anchorHandle.height / 2,
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
        final contentWidth = geo.contentWidth;
        final needsHorizontalScroll = contentWidth > viewWidth;
        final hasRangeSelection = !(state.state.selection?.collapsed ?? true);
        final horizontalPhysics = isSelecting || !needsHorizontalScroll
            ? const NeverScrollableScrollPhysics()
            : const _NonGestureBouncingScrollPhysics();

        final contentBottomPadding = geo.bottomPadding(
          viewportHeight: verticalScrollController.hasSingleClient
              ? verticalScrollController.position.viewportDimension
              : viewHeight,
          cursor: cursor,
          typewriterEnabled: pref.typewriterEnabled,
          typewriterPosition: pref.typewriterPosition,
        );

        void startLongPress(Offset globalPosition) {
          if (gesture.isCellHandleDragging) {
            return;
          }
          if (gestureState.active) {
            return;
          }
          final viewportPosition = viewportPositionFromGlobal(globalPosition);
          if (viewportPosition == null) {
            return;
          }
          scope.inputController.commitComposing();

          longPressPosition.value = viewportPosition;
          if (!gestureState.startLongPress()) {
            return;
          }

          final draggingHandle = state.state.draggingHandle;
          final anchorHandle = draggingHandle == SelectionHandleType.from
              ? state.state.selection?.toBounds
              : state.state.selection?.fromBounds;

          gesture.beginLongPressSession(
            touchPosition: globalPosition,
            handleScreenPosition: gesture.getHandleStemCenter(fromHandle ?? toHandle, scope.geometry),
            anchorHandle: anchorHandle,
          );
        }

        void updateLongPress(Offset viewportPosition) {
          if (!gestureState.longPressing || gestureState.active) {
            return;
          }

          final (pageIdx, localY) = getPageAtPosition(viewportPosition.dy);
          longPressPosition.value = viewportPosition;

          if (pageIdx >= 0) {
            final pointerX = gesture.getPointerX(viewportPosition.dx);
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

        void endLongPress() {
          if (!gestureState.longPressing || gestureState.active) {
            return;
          }
          longPressPosition.value = null;
          gesture.stopAutoScroll();
          gestureState.endLongPress();
        }

        final listView = ScrollConfiguration(
          behavior: ScrollConfiguration.of(context).copyWith(scrollbars: false, dragDevices: isSelecting ? {} : null),
          child: SingleChildScrollView(
            controller: verticalScrollController,
            physics: isSelecting ? const NeverScrollableScrollPhysics() : const _NonGestureBouncingScrollPhysics(),
            child: Builder(
              builder: (_) {
                final content = RawGestureDetector(
                  gestures: {
                    ConditionalLongPressGestureRecognizer:
                        GestureRecognizerFactoryWithHandlers<ConditionalLongPressGestureRecognizer>(
                          () => ConditionalLongPressGestureRecognizer(
                            condition: (globalPosition) {
                              if (gesture.isCellHandleDragging) {
                                return false;
                              }
                              if (gestureState.active) {
                                return true;
                              }
                              final viewportPosition = viewportPositionFromGlobal(globalPosition);
                              if (viewportPosition == null) {
                                return true;
                              }
                              final (pageIdx, localY) = getPageAtPosition(viewportPosition.dy);
                              final pointerX = gesture.getPointerX(viewportPosition.dx);
                              return scope.editor.isSelectionHit(pageIdx, pointerX, localY);
                            },
                            duration: const Duration(milliseconds: 500),
                          ),
                          (ConditionalLongPressGestureRecognizer instance) {
                            instance
                              ..onLongPressStart = (details) {
                                startLongPress(details.globalPosition);
                              }
                              ..onLongPressEnd = (details) {
                                endLongPress();
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
                            width: (geo.pages.firstOrNull?.width ?? 0) + geo.horizontalPadding * 2,
                            padding: EdgeInsets.only(
                              left: geo.horizontalPadding,
                              right: geo.horizontalPadding,
                              bottom: contentBottomPadding,
                            ),
                            child: Column(
                              children: [
                                for (var i = 0; i < pages.length; i++) ...[
                                  _PageSlot(
                                    key: ValueKey(i),
                                    pageIndex: i,
                                    pageTop: geo.titleAreaHeight + offsets[i],
                                    pageBottom: geo.titleAreaHeight + offsets[i] + pages[i].height,
                                  ),
                                ],
                              ],
                            ),
                          ),
                        )
                      else
                        Container(
                          width: (geo.pages.firstOrNull?.width ?? 0) + geo.horizontalPadding * 2,
                          padding: EdgeInsets.only(
                            left: geo.horizontalPadding,
                            right: geo.horizontalPadding,
                            bottom: contentBottomPadding,
                          ),
                          child: Column(
                            children: [
                              for (var i = 0; i < pages.length; i++) ...[
                                _PageSlot(
                                  key: ValueKey(i),
                                  pageIndex: i,
                                  pageTop: geo.titleAreaHeight + offsets[i],
                                  pageBottom: geo.titleAreaHeight + offsets[i] + pages[i].height,
                                ),
                              ],
                            ],
                          ),
                        ),
                    ],
                  ),
                );

                return EditorDraggable(gesture: gesture, child: content);
              },
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
          final clickCount = isConsecutiveTap(localPosition: localPosition, now: now) ? 2 : 1;

          gesture.recordTap(now: now, localPosition: localPosition);

          final pointerX = gesture.getPointerX(localPosition.dx);
          final tappedInteractive = scope.editor.isInteractiveHit(pageIdx, pointerX, localY);

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

          if (clickCount != 1) {
            scope.controller.scrollIntoView();
            return;
          }

          unawaited(
            scope.ticker.settled().then((_) {
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
                if (!tappedInteractive && !wasContextMenuOpen.value) {
                  showContextMenu.value = true;
                }
                return;
              }

              if (tappedInteractive) {
                return;
              }

              scope.controller.scrollIntoView();
            }),
          );
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
            wasContextMenuOpen.value = showContextMenu.value;
            if (showContextMenu.value) {
              showContextMenu.value = false;
            }

            gesture.cancelTapTimer();

            if (isConsecutiveTap(localPosition: details.localPosition, now: DateTime.now())) {
              gesture.setTapDispatched(true);
              if (dispatchDoubleTapSelection(details.localPosition)) {
                prepareDoubleTapDrag(details.localPosition);
              }
              return;
            }

            gesture
              ..setTapDispatched(false)
              ..scheduleTapTimer(const Duration(milliseconds: 150), () {
                final pointerX = gesture.getPointerX(details.localPosition.dx);
                final (pageIdx, localY) = getPageAtPosition(details.localPosition.dy);

                final canDrag = scope.editor.isSelectionHit(pageIdx, pointerX, localY);

                if (canDrag) {
                  gesture.setTapDispatched(true);
                  return;
                }

                if (hasRangeSelection) {
                  // Keep current selection until long-press gesture resolves.
                  return;
                }

                gesture.setTapDispatched(true);
                dispatchTap(details.localPosition);
              });
          },
          onTapUp: (details) {
            if (gestureState.dragging) {
              return;
            }
            gestureState.clearPending();
            gesture.cancelTapTimer();
            if (!gesture.tapDispatched) {
              dispatchTap(details.localPosition);
            }
          },
          onTapCancel: () {
            if (gestureState.dragging) {
              return;
            }
            gestureState.clearPending();
            gesture.cancelTapTimer();
          },

          onPanDown: (details) {
            if (isSelecting) {
              return;
            }
            gesture.holdScrollPositions();
          },
          onPanStart: (details) {
            if (gestureState.active) {
              return;
            }
            gesture.startScrollDrag(details: details, allowHorizontal: needsHorizontalScroll);
          },
          onPanUpdate: gesture.updateScrollDrag,
          onPanEnd: (details) {
            if (gestureState.active) {
              return;
            }
            gesture.endScrollDrag(details);
          },
          onPanCancel: () {
            if (gestureState.active) {
              return;
            }
            gesture.cancelScrollDrag();
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
            onPointerMove: (event) {
              if (gestureState.pending) {
                final startPosition = gestureState.start;
                if (startPosition != null && (event.localPosition - startPosition).distance >= 4) {
                  startDoubleTapDrag(startPosition);
                  updateDoubleTapDragSelection(event.localPosition);
                }
                return;
              }
              if (gestureState.dragging) {
                updateDoubleTapDragSelection(event.localPosition);
                return;
              }

              if (gestureState.longPressing) {
                updateLongPress(event.localPosition);
              }
            },
            onPointerUp: (_) {
              if (gestureState.dragging) {
                endDoubleTapDrag();
                return;
              }
              gestureState.clearPending();
              endLongPress();
              endTextHandleDrag();
            },
            onPointerCancel: (_) {
              if (gestureState.dragging) {
                endDoubleTapDrag();
                return;
              }
              gestureState.clearPending();
              endLongPress();
              endTextHandleDrag();
            },
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
                if (showContextMenu.value &&
                    longPressPosition.value == null &&
                    !gesture.hasTextHandleDrag &&
                    !gestureState.active)
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
      if (!verticalScrollController.hasSingleClient) {
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
