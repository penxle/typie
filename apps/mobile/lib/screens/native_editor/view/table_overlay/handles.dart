import 'package:flutter/material.dart';
import 'package:typie/context/theme.dart';

import 'constants.dart';

class SelectorHandleButton extends StatelessWidget {
  const SelectorHandleButton({
    required this.width,
    required this.height,
    required this.icon,
    required this.onTap,
    super.key,
  });

  final double width;
  final double height;
  final IconData icon;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: onTap,
      child: Container(
        width: width,
        height: height,
        decoration: BoxDecoration(
          color: context.colors.surfaceDefault,
          borderRadius: BorderRadius.circular(6),
          border: Border.all(color: context.colors.borderStrong),
          boxShadow: [
            BoxShadow(color: Colors.black.withValues(alpha: 0.08), offset: const Offset(0, 1), blurRadius: 4),
          ],
        ),
        child: Icon(icon, size: 14, color: context.colors.textSubtle),
      ),
    );
  }
}

class CellSelectionHandle extends StatelessWidget {
  const CellSelectionHandle({
    required this.color,
    required this.onPanDown,
    required this.onPanStart,
    required this.onPanUpdate,
    required this.onPanEnd,
    required this.onPanCancel,
    super.key,
  });

  final Color color;
  final GestureDragDownCallback onPanDown;
  final GestureDragStartCallback onPanStart;
  final GestureDragUpdateCallback onPanUpdate;
  final GestureDragEndCallback onPanEnd;
  final VoidCallback onPanCancel;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onPanDown: onPanDown,
      onPanStart: onPanStart,
      onPanUpdate: onPanUpdate,
      onPanEnd: onPanEnd,
      onPanCancel: onPanCancel,
      child: SizedBox(
        width: tableCellSelectionHandleTouchSize,
        height: tableCellSelectionHandleTouchSize,
        child: Center(
          child: Container(
            width: tableCellSelectionHandleRadius * 2,
            height: tableCellSelectionHandleRadius * 2,
            decoration: BoxDecoration(
              color: color,
              shape: BoxShape.circle,
              boxShadow: [
                BoxShadow(color: Colors.black.withValues(alpha: 0.12), blurRadius: 4, offset: const Offset(0, 1)),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
