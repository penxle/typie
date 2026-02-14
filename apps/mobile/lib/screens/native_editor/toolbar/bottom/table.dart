import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/services/keyboard.dart';
import 'package:typie/widgets/tappable.dart';

const _minTableSize = 1;
const _maxTableSize = 10;
const _gridCellSize = 36.0;
const _gridGap = 4.0;
const _gridOuterPadding = _gridGap;
const _gridEdgeFadeSize = 20.0;

class NativeEditorTableSizeBottomToolbar extends HookWidget {
  const NativeEditorTableSizeBottomToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final keyboardType = useValueListenable(scope.keyboardType);
    final selectedRows = useState(3);
    final selectedCols = useState(3);

    final verticalController = useScrollController();
    final horizontalController = useScrollController();
    useListenable(verticalController);
    useListenable(horizontalController);
    final activeTouchViewport = useRef<Offset?>(null);
    final holdResampleTimer = useRef<Timer?>(null);

    useEffect(() {
      return () {
        holdResampleTimer.value?.cancel();
      };
    }, const []);

    void insertTable() {
      scope.dispatch({'type': 'insertTable', 'rows': selectedRows.value, 'cols': selectedCols.value});
      scope.controller.scrollIntoView();

      switch (keyboardType) {
        case KeyboardType.software:
          scope.requestFocus();
        case KeyboardType.hardware:
          scope.bottomToolbarMode.value = BottomToolbarMode.hidden;
      }
    }

    void centerToCell({
      required int rowIndex,
      required int colIndex,
      required double viewportWidth,
      required double viewportHeight,
      bool animated = true,
      Duration duration = const Duration(milliseconds: 180),
      Curve curve = Curves.easeOutCubic,
    }) {
      if (!verticalController.hasClients || !horizontalController.hasClients) {
        return;
      }

      const stride = _gridCellSize + _gridGap;
      final cellCenterX = _gridOuterPadding + colIndex * stride + _gridCellSize / 2;
      final cellCenterY = _gridOuterPadding + rowIndex * stride + _gridCellSize / 2;

      final targetX = (cellCenterX - viewportWidth / 2).clamp(0.0, horizontalController.position.maxScrollExtent);
      final targetY = (cellCenterY - viewportHeight / 2).clamp(0.0, verticalController.position.maxScrollExtent);

      if (!animated) {
        horizontalController.jumpTo(targetX);
        verticalController.jumpTo(targetY);
        return;
      }

      unawaited(horizontalController.animateTo(targetX, duration: duration, curve: curve));
      unawaited(verticalController.animateTo(targetY, duration: duration, curve: curve));
    }

    const gridContentExtent = _maxTableSize * _gridCellSize + (_maxTableSize - 1) * _gridGap;
    const gridExtent = gridContentExtent + _gridOuterPadding * 2;

    return LayoutBuilder(
      builder: (context, constraints) {
        final safeBottom = MediaQuery.paddingOf(context).bottom;
        final viewportWidth = constraints.maxWidth;
        final viewportHeight = constraints.maxHeight > safeBottom ? constraints.maxHeight - safeBottom : 0.0;
        final hasLeftFade = horizontalController.hasClients && horizontalController.offset > 0.5;
        final hasRightFade =
            horizontalController.hasClients &&
            horizontalController.offset < horizontalController.position.maxScrollExtent - 0.5;
        final hasTopFade = verticalController.hasClients && verticalController.offset > 0.5;
        final hasBottomFade =
            verticalController.hasClients &&
            verticalController.offset < verticalController.position.maxScrollExtent - 0.5;

        void selectSize(
          int rows,
          int cols, {
          bool center = true,
          bool centerAnimated = true,
          Duration centerDuration = const Duration(milliseconds: 180),
          Curve centerCurve = Curves.easeOutCubic,
        }) {
          final nextRows = rows.clamp(_minTableSize, _maxTableSize);
          final nextCols = cols.clamp(_minTableSize, _maxTableSize);

          if (nextRows == selectedRows.value && nextCols == selectedCols.value) {
            return;
          }

          selectedRows.value = nextRows;
          selectedCols.value = nextCols;

          if (!center) {
            return;
          }

          WidgetsBinding.instance.addPostFrameCallback((_) {
            centerToCell(
              rowIndex: nextRows - 1,
              colIndex: nextCols - 1,
              viewportWidth: viewportWidth,
              viewportHeight: viewportHeight,
              animated: centerAnimated,
              duration: centerDuration,
              curve: centerCurve,
            );
          });
        }

        void selectFromGridPosition(
          Offset gridPosition, {
          bool center = true,
          bool centerAnimated = true,
          Duration centerDuration = const Duration(milliseconds: 180),
          Curve centerCurve = Curves.easeOutCubic,
        }) {
          const stride = _gridCellSize + _gridGap;
          final col = ((gridPosition.dx - _gridOuterPadding) / stride).floor();
          final row = ((gridPosition.dy - _gridOuterPadding) / stride).floor();

          final clampedCol = col < 0
              ? 0
              : col >= _maxTableSize
              ? _maxTableSize - 1
              : col;
          final clampedRow = row < 0
              ? 0
              : row >= _maxTableSize
              ? _maxTableSize - 1
              : row;

          selectSize(
            clampedRow + 1,
            clampedCol + 1,
            center: center,
            centerAnimated: centerAnimated,
            centerDuration: centerDuration,
            centerCurve: centerCurve,
          );
        }

        void selectFromViewportPosition(
          Offset viewportPosition, {
          Duration centerDuration = const Duration(milliseconds: 180),
          Curve centerCurve = Curves.easeOutCubic,
        }) {
          activeTouchViewport.value = viewportPosition;
          final gridPosition = Offset(
            viewportPosition.dx + (horizontalController.hasClients ? horizontalController.offset : 0),
            viewportPosition.dy + (verticalController.hasClients ? verticalController.offset : 0),
          );
          selectFromGridPosition(gridPosition, centerDuration: centerDuration, centerCurve: centerCurve);
        }

        void startHoldResample() {
          holdResampleTimer.value?.cancel();
          holdResampleTimer.value = Timer.periodic(const Duration(milliseconds: 90), (_) {
            final viewportPosition = activeTouchViewport.value;
            if (viewportPosition == null) {
              return;
            }
            final gridPosition = Offset(
              viewportPosition.dx + (horizontalController.hasClients ? horizontalController.offset : 0),
              viewportPosition.dy + (verticalController.hasClients ? verticalController.offset : 0),
            );
            selectFromGridPosition(
              gridPosition,
              centerDuration: const Duration(milliseconds: 90),
              centerCurve: Curves.linear,
            );
          });
        }

        void stopHoldResample() {
          holdResampleTimer.value?.cancel();
          holdResampleTimer.value = null;
          activeTouchViewport.value = null;
        }

        return Padding(
          padding: Pad(bottom: safeBottom),
          child: Stack(
            children: [
              Positioned.fill(
                child: SingleChildScrollView(
                  controller: verticalController,
                  physics: const NeverScrollableScrollPhysics(),
                  child: SingleChildScrollView(
                    controller: horizontalController,
                    scrollDirection: Axis.horizontal,
                    physics: const NeverScrollableScrollPhysics(),
                    child: SizedBox(
                      width: gridExtent,
                      height: gridExtent,
                      child: Padding(
                        padding: const EdgeInsets.all(_gridOuterPadding),
                        child: Column(
                          children: [
                            for (var row = 0; row < _maxTableSize; row++) ...[
                              Row(
                                children: [
                                  for (var col = 0; col < _maxTableSize; col++) ...[
                                    AnimatedContainer(
                                      duration: const Duration(milliseconds: 80),
                                      curve: Curves.easeOut,
                                      width: _gridCellSize,
                                      height: _gridCellSize,
                                      decoration: BoxDecoration(
                                        color: row < selectedRows.value && col < selectedCols.value
                                            ? context.colors.accentBrand.withValues(alpha: 0.25)
                                            : context.colors.surfaceDefault,
                                        border: Border.all(
                                          color: row < selectedRows.value && col < selectedCols.value
                                              ? context.colors.accentBrand
                                              : context.colors.borderDefault,
                                        ),
                                        borderRadius: BorderRadius.circular(4),
                                      ),
                                    ),
                                    if (col < _maxTableSize - 1) const SizedBox(width: _gridGap),
                                  ],
                                ],
                              ),
                              if (row < _maxTableSize - 1) const SizedBox(height: _gridGap),
                            ],
                          ],
                        ),
                      ),
                    ),
                  ),
                ),
              ),
              Positioned.fill(
                child: GestureDetector(
                  behavior: HitTestBehavior.opaque,
                  onPanDown: (details) {
                    selectFromViewportPosition(
                      details.localPosition,
                      centerDuration: const Duration(milliseconds: 90),
                      centerCurve: Curves.linear,
                    );
                    startHoldResample();
                  },
                  onTapUp: (_) {
                    stopHoldResample();
                  },
                  onTapCancel: stopHoldResample,
                  onPanStart: (details) {
                    selectFromViewportPosition(
                      details.localPosition,
                      centerDuration: const Duration(milliseconds: 90),
                      centerCurve: Curves.linear,
                    );
                  },
                  onPanUpdate: (details) {
                    selectFromViewportPosition(
                      details.localPosition,
                      centerDuration: const Duration(milliseconds: 90),
                      centerCurve: Curves.linear,
                    );
                  },
                  onPanEnd: (_) {
                    stopHoldResample();
                    centerToCell(
                      rowIndex: selectedRows.value - 1,
                      colIndex: selectedCols.value - 1,
                      viewportWidth: viewportWidth,
                      viewportHeight: viewportHeight,
                    );
                  },
                  onPanCancel: stopHoldResample,
                ),
              ),
              IgnorePointer(
                child: Stack(
                  children: [
                    Positioned(
                      left: 0,
                      top: 0,
                      right: 0,
                      height: _gridEdgeFadeSize,
                      child: AnimatedOpacity(
                        opacity: hasTopFade ? 1 : 0,
                        duration: const Duration(milliseconds: 160),
                        curve: Curves.easeOutCubic,
                        child: DecoratedBox(
                          decoration: BoxDecoration(
                            gradient: LinearGradient(
                              begin: Alignment.topCenter,
                              end: Alignment.bottomCenter,
                              colors: [
                                context.colors.surfaceDefault,
                                context.colors.surfaceDefault.withValues(alpha: 0),
                              ],
                            ),
                          ),
                        ),
                      ),
                    ),
                    Positioned(
                      left: 0,
                      bottom: 0,
                      right: 0,
                      height: _gridEdgeFadeSize,
                      child: AnimatedOpacity(
                        opacity: hasBottomFade ? 1 : 0,
                        duration: const Duration(milliseconds: 160),
                        curve: Curves.easeOutCubic,
                        child: DecoratedBox(
                          decoration: BoxDecoration(
                            gradient: LinearGradient(
                              begin: Alignment.bottomCenter,
                              end: Alignment.topCenter,
                              colors: [
                                context.colors.surfaceDefault,
                                context.colors.surfaceDefault.withValues(alpha: 0),
                              ],
                            ),
                          ),
                        ),
                      ),
                    ),
                    Positioned(
                      left: 0,
                      top: 0,
                      bottom: 0,
                      width: _gridEdgeFadeSize,
                      child: AnimatedOpacity(
                        opacity: hasLeftFade ? 1 : 0,
                        duration: const Duration(milliseconds: 160),
                        curve: Curves.easeOutCubic,
                        child: DecoratedBox(
                          decoration: BoxDecoration(
                            gradient: LinearGradient(
                              colors: [
                                context.colors.surfaceDefault,
                                context.colors.surfaceDefault.withValues(alpha: 0),
                              ],
                            ),
                          ),
                        ),
                      ),
                    ),
                    Positioned(
                      right: 0,
                      top: 0,
                      bottom: 0,
                      width: _gridEdgeFadeSize,
                      child: AnimatedOpacity(
                        opacity: hasRightFade ? 1 : 0,
                        duration: const Duration(milliseconds: 160),
                        curve: Curves.easeOutCubic,
                        child: DecoratedBox(
                          decoration: BoxDecoration(
                            gradient: LinearGradient(
                              begin: Alignment.centerRight,
                              end: Alignment.centerLeft,
                              colors: [
                                context.colors.surfaceDefault,
                                context.colors.surfaceDefault.withValues(alpha: 0),
                              ],
                            ),
                          ),
                        ),
                      ),
                    ),
                  ],
                ),
              ),
              Positioned(
                right: 12,
                bottom: 12,
                child: Tappable(
                  onTap: insertTable,
                  child: Container(
                    decoration: BoxDecoration(
                      border: Border.all(color: context.colors.borderStrong),
                      borderRadius: BorderRadius.circular(999),
                      color: context.colors.surfaceDefault,
                      boxShadow: [
                        BoxShadow(
                          color: Colors.black.withValues(alpha: 0.08),
                          offset: const Offset(0, 2),
                          blurRadius: 8,
                        ),
                      ],
                    ),
                    padding: const Pad(horizontal: 14, vertical: 10),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      spacing: 4,
                      children: [
                        Icon(LucideLightIcons.table, size: 16, color: context.colors.textSubtle),
                        Text(
                          '${selectedRows.value}×${selectedCols.value} 삽입',
                          style: TextStyle(
                            fontSize: 13,
                            fontWeight: FontWeight.w600,
                            color: context.colors.textDefault,
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ],
          ),
        );
      },
    );
  }
}
