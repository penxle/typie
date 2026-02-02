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
    final isVisible = useState(true);
    final blinkTimer = useRef<Timer?>(null);

    final cursor = cursorInfo;
    final shouldAnimate = cursor != null && cursor.show && isFocused;

    void startBlinkTimer() {
      blinkTimer.value?.cancel();
      isVisible.value = true;
      blinkTimer.value = Timer.periodic(const Duration(milliseconds: 500), (_) {
        isVisible.value = !isVisible.value;
      });
    }

    useEffect(() {
      if (!shouldAnimate) {
        blinkTimer.value?.cancel();
        return null;
      }

      startBlinkTimer();

      return () {
        blinkTimer.value?.cancel();
      };
    }, [shouldAnimate]);

    useEffect(() {
      if (cursor == null || !shouldAnimate) {
        return null;
      }

      startBlinkTimer();

      return null;
    }, [cursorInfo?.x, cursorInfo?.y]);

    if (!shouldAnimate || !isVisible.value) {
      return const SizedBox.shrink();
    }

    return Positioned(
      left: cursor.x,
      top: cursor.y,
      child: Container(width: 1, height: cursor.height, color: context.colors.textDefault),
    );
  }
}
