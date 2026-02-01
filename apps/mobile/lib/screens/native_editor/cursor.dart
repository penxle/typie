import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';

class CursorInfo {
  const CursorInfo({
    required this.pageIdx,
    required this.x,
    required this.y,
    required this.height,
    required this.show,
    required this.scrollToCursor,
    required this.animate,
  });

  factory CursorInfo.fromMap(Map<String, dynamic> map) {
    final bounds = map['bounds'] as Map<String, dynamic>?;
    return CursorInfo(
      pageIdx: map['pageIdx'] as int? ?? 0,
      x: (bounds?['x'] as num?)?.toDouble() ?? 0,
      y: (bounds?['y'] as num?)?.toDouble() ?? 0,
      height: (bounds?['height'] as num?)?.toDouble() ?? 0,
      show: map['show'] as bool? ?? false,
      scrollToCursor: map['scrollToCursor'] as bool? ?? false,
      animate: map['animate'] as bool? ?? false,
    );
  }

  final int pageIdx;
  final double x;
  final double y;
  final double height;
  final bool show;
  final bool scrollToCursor;
  final bool animate;

  CursorInfo copyWith({
    int? pageIdx,
    double? x,
    double? y,
    double? height,
    bool? show,
    bool? scrollToCursor,
    bool? animate,
  }) {
    return CursorInfo(
      pageIdx: pageIdx ?? this.pageIdx,
      x: x ?? this.x,
      y: y ?? this.y,
      height: height ?? this.height,
      show: show ?? this.show,
      scrollToCursor: scrollToCursor ?? this.scrollToCursor,
      animate: animate ?? this.animate,
    );
  }
}

class LineHighlight extends StatelessWidget {
  const LineHighlight({required this.cursorInfo, required this.isFocused, required this.enabled, super.key});

  final CursorInfo? cursorInfo;
  final bool isFocused;
  final bool enabled;

  static const double _padding = 4;

  @override
  Widget build(BuildContext context) {
    final cursor = cursorInfo;
    if (!enabled || cursor == null || !cursor.show || !isFocused) {
      return const SizedBox.shrink();
    }

    return Positioned(
      left: 0,
      right: 0,
      top: cursor.y - _padding,
      height: cursor.height + _padding * 2,
      child: Container(color: context.colors.surfaceMuted),
    );
  }
}

class EditorCursor extends HookWidget {
  const EditorCursor({required this.cursorInfo, required this.isFocused, super.key});

  final CursorInfo? cursorInfo;
  final bool isFocused;

  @override
  Widget build(BuildContext context) {
    final controller = useAnimationController(duration: const Duration(milliseconds: 1000));
    final prevPosition = useRef<(double, double)?>(null);

    useEffect(() {
      unawaited(controller.repeat());
      return null;
    }, []);

    useEffect(() {
      final cursor = cursorInfo;
      if (cursor == null) {
        return null;
      }

      final currentPos = (cursor.x, cursor.y);
      if (prevPosition.value != currentPos) {
        prevPosition.value = currentPos;
        controller.value = 0;
        unawaited(controller.repeat());
      }

      return null;
    }, [cursorInfo?.x, cursorInfo?.y]);

    final cursor = cursorInfo;
    if (cursor == null || !cursor.show || !isFocused) {
      return const SizedBox.shrink();
    }

    return Positioned(
      left: cursor.x,
      top: cursor.y,
      child: AnimatedBuilder(
        animation: controller,
        builder: (context, child) {
          final opacity = controller.value < 0.5 ? 1.0 : 0.0;
          return Opacity(opacity: opacity, child: child);
        },
        child: Container(width: 1, height: cursor.height, color: context.colors.textDefault),
      ),
    );
  }
}
