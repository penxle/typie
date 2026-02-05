import 'package:flutter/material.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/screens/native_editor/state/state.dart';

class SelectionHandle extends StatelessWidget {
  const SelectionHandle({
    required this.handleInfo,
    required this.type,
    required this.onDragDown,
    required this.onDragStart,
    required this.onDragUpdate,
    required this.onDragEnd,
    super.key,
  });

  final SelectionHandleInfo handleInfo;
  final SelectionHandleType type;
  final void Function(SelectionHandleType, DragDownDetails) onDragDown;
  final void Function(SelectionHandleType, DragStartDetails) onDragStart;
  final void Function(SelectionHandleType, DragUpdateDetails) onDragUpdate;
  final void Function(SelectionHandleType, DragEndDetails) onDragEnd;

  static const double handleRadius = 8;
  static const double stemWidth = 2;
  static const double touchTargetSize = 44;

  Offset get offset {
    final isFrom = type == SelectionHandleType.from;
    final stemHeight = handleInfo.height;
    final totalHeight = handleRadius * 2 + stemHeight;
    final effectiveTouchHeight = totalHeight > touchTargetSize ? totalHeight : touchTargetSize;

    final customPaintTop = isFrom ? -(handleRadius * 2) : 0.0;

    final handleCenterY = customPaintTop + totalHeight / 2;
    final touchTargetTop = handleCenterY - effectiveTouchHeight / 2;

    final handleXOffset = isFrom ? -stemWidth / 2 : stemWidth / 2;
    final touchTargetLeft = handleXOffset - touchTargetSize / 2;

    return Offset(touchTargetLeft, touchTargetTop);
  }

  Offset get _customPaintOffset {
    final isFrom = type == SelectionHandleType.from;
    final stemHeight = handleInfo.height;
    final totalHeight = handleRadius * 2 + stemHeight;
    final effectiveTouchHeight = totalHeight > touchTargetSize ? totalHeight : touchTargetSize;

    final customPaintTop = isFrom ? -(handleRadius * 2) : 0.0;
    final handleCenterY = customPaintTop + totalHeight / 2;
    final touchTargetTop = handleCenterY - effectiveTouchHeight / 2;

    final relativeTop = customPaintTop - touchTargetTop;
    const relativeLeft = (touchTargetSize - handleRadius * 2) / 2;

    return Offset(relativeLeft, relativeTop);
  }

  double get _effectiveTouchHeight {
    final stemHeight = handleInfo.height;
    final totalHeight = handleRadius * 2 + stemHeight;
    return totalHeight > touchTargetSize ? totalHeight : touchTargetSize;
  }

  @override
  Widget build(BuildContext context) {
    final stemHeight = handleInfo.height;
    final totalHeight = handleRadius * 2 + stemHeight;

    return Transform.translate(
      offset: offset,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onPanDown: (details) => onDragDown(type, details),
        onPanStart: (details) => onDragStart(type, details),
        onPanUpdate: (details) => onDragUpdate(type, details),
        onPanEnd: (details) => onDragEnd(type, details),
        onLongPressStart: (details) => onDragStart(
          type,
          DragStartDetails(localPosition: details.localPosition, globalPosition: details.globalPosition),
        ),
        onLongPressMoveUpdate: (details) => onDragUpdate(
          type,
          DragUpdateDetails(localPosition: details.localPosition, globalPosition: details.globalPosition),
        ),
        onLongPressEnd: (details) => onDragEnd(
          type,
          DragEndDetails(localPosition: details.localPosition, globalPosition: details.globalPosition),
        ),
        child: SizedBox(
          width: touchTargetSize,
          height: _effectiveTouchHeight,
          child: Stack(
            clipBehavior: Clip.none,
            children: [
              Positioned(
                left: _customPaintOffset.dx,
                top: _customPaintOffset.dy,
                child: CustomPaint(
                  size: Size(handleRadius * 2, totalHeight),
                  painter: _SelectionHandlePainter(
                    type: type,
                    stemHeight: stemHeight,
                    color: context.colors.textDefault,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _SelectionHandlePainter extends CustomPainter {
  _SelectionHandlePainter({required this.type, required this.stemHeight, required this.color});

  final SelectionHandleType type;
  final double stemHeight;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = color
      ..style = PaintingStyle.fill;

    const radius = SelectionHandle.handleRadius;
    const stemWidth = SelectionHandle.stemWidth;
    final centerX = size.width / 2;

    if (type == SelectionHandleType.from) {
      canvas
        ..drawCircle(Offset(centerX, radius), radius, paint)
        ..drawRect(Rect.fromLTWH(centerX - stemWidth / 2, radius * 2, stemWidth, stemHeight), paint);
    } else {
      canvas
        ..drawRect(Rect.fromLTWH(centerX - stemWidth / 2, 0, stemWidth, stemHeight), paint)
        ..drawCircle(Offset(centerX, stemHeight + radius), radius, paint);
    }
  }

  @override
  bool shouldRepaint(_SelectionHandlePainter oldDelegate) {
    return type != oldDelegate.type || stemHeight != oldDelegate.stemHeight || color != oldDelegate.color;
  }
}
