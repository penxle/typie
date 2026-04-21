import 'package:flutter/material.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/screens/native_editor/state/state.dart';

class LineHighlight extends StatelessWidget {
  const LineHighlight({required this.cursorInfo, required this.isFocused, required this.enabled, super.key});

  final CursorInfo? cursorInfo;
  final bool isFocused;
  final bool enabled;

  static const double _padding = 4;
  static const double _horizontalOverflow = 100000;

  @override
  Widget build(BuildContext context) {
    final cursor = cursorInfo;
    if (!enabled || cursor == null || !cursor.visible || !isFocused) {
      return const SizedBox.shrink();
    }

    return Positioned(
      left: -_horizontalOverflow,
      width: _horizontalOverflow * 2,
      top: cursor.y - _padding,
      height: cursor.height + _padding * 2,
      child: Container(color: context.colors.surfaceMuted),
    );
  }
}
