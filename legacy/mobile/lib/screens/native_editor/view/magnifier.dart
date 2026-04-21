import 'package:flutter/material.dart';
import 'package:typie/context/theme.dart';

class EditorMagnifier extends StatelessWidget {
  const EditorMagnifier({
    required this.position,
    required this.focalPoint,
    required this.visibleOrigin,
    required this.visibleSize,
    super.key,
  });

  final Offset position;
  final Offset focalPoint;
  final Offset visibleOrigin;
  final Size visibleSize;

  static const Size _size = Size(144, 80);
  static const double _magnification = 1.3;
  static const double _verticalOffset = 60;

  @override
  Widget build(BuildContext context) {
    final clampedX = position.dx.clamp(_size.width / 2, visibleSize.width - _size.width / 2);
    final showBelow = position.dy < _size.height + _verticalOffset;
    final preferredY = showBelow ? position.dy + _verticalOffset : position.dy - _verticalOffset - _size.height;
    final magnifierPosition = Offset(clampedX - _size.width / 2, preferredY < 0 ? 0 : preferredY);

    final borderRadius = BorderRadius.circular(_size.height / 2);

    return Positioned(
      left: visibleOrigin.dx + magnifierPosition.dx,
      top: visibleOrigin.dy + magnifierPosition.dy,
      child: IgnorePointer(
        child: Container(
          decoration: BoxDecoration(
            borderRadius: borderRadius,
            boxShadow: [
              BoxShadow(
                color: context.colors.shadowDefault.withValues(alpha: 0.26),
                blurRadius: 8,
                offset: const Offset(0, 2),
              ),
            ],
          ),
          child: RawMagnifier(
            size: _size,
            magnificationScale: _magnification,
            focalPointOffset: Offset(
              focalPoint.dx - magnifierPosition.dx - _size.width / 2,
              focalPoint.dy - magnifierPosition.dy - _size.height / 2,
            ),
            decoration: MagnifierDecoration(
              shape: RoundedRectangleBorder(
                borderRadius: borderRadius,
                side: BorderSide(color: context.colors.borderDefault),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
