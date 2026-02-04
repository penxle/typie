import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/screens/native_editor/state/state.dart';

class Cursor extends HookWidget {
  const Cursor({required this.cursorInfo, required this.isFocused, super.key});

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
