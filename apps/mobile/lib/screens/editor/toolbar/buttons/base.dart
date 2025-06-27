import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';

enum _ButtonState { idle, pressed, active }

class ToolbarButton extends HookWidget {
  const ToolbarButton({
    required this.onTap,
    required this.builder,
    this.isActive = false,
    this.isRepeatable = false,
    this.color,
    super.key,
  });

  final Widget Function(BuildContext context, Color color, Color? backgroundColor) builder;

  final Color? color;
  final bool isActive;
  final bool isRepeatable;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    final state = useState(_ButtonState.idle);
    final effectiveState = state.value == _ButtonState.pressed
        ? _ButtonState.pressed
        : isActive
        ? _ButtonState.active
        : _ButtonState.idle;

    final controller = useAnimationController(duration: const Duration(milliseconds: 150));
    final curve = useMemoized(() => CurvedAnimation(parent: controller, curve: Curves.ease), [controller]);

    final defaultForegroundColor = isActive ? context.colors.textDefault : (color ?? context.colors.textSubtle);
    final foregroundTween = useRef<ColorTween?>(null);
    final backgroundTween = useRef<ColorTween?>(null);

    final repeatTimer = useRef<Timer?>(null);

    final textSubtleColor = context.colors.textSubtle;
    final borderDefaultColor = context.colors.borderDefault;
    final textDefaultColor = context.colors.textDefault;
    final surfaceDefaultColor = context.colors.surfaceDefault;
    final surfaceSubtleColor = context.colors.surfaceSubtle;

    useEffect(() {
      foregroundTween.value = ColorTween(
        begin: foregroundTween.value?.evaluate(curve) ?? defaultForegroundColor,
        end: switch (effectiveState) {
          _ButtonState.idle => color ?? textSubtleColor,
          _ButtonState.pressed => borderDefaultColor,
          _ButtonState.active => textDefaultColor,
        },
      );

      backgroundTween.value = ColorTween(
        begin: backgroundTween.value?.evaluate(curve),
        end: switch (effectiveState) {
          _ButtonState.idle => surfaceDefaultColor,
          _ButtonState.pressed => surfaceDefaultColor,
          _ButtonState.active => surfaceSubtleColor,
        },
      );

      controller.forward(from: 0);

      return null;
    }, [effectiveState]);

    useEffect(() {
      return repeatTimer.value?.cancel;
    }, []);

    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: onTap,
      onLongPressStart: (_) {
        state.value = _ButtonState.pressed;
        if (isRepeatable) {
          repeatTimer.value = Timer.periodic(const Duration(milliseconds: 100), (_) {
            onTap();
          });
        }
      },
      onLongPressEnd: (_) {
        repeatTimer.value?.cancel();
        state.value = _ButtonState.idle;
      },
      onTapDown: (_) => state.value = _ButtonState.pressed,
      onTapUp: (_) => state.value = _ButtonState.idle,
      onTapCancel: () => state.value = _ButtonState.idle,
      child: AnimatedBuilder(
        animation: controller,
        builder: (context, child) {
          final foregroundColor = foregroundTween.value?.evaluate(curve) ?? defaultForegroundColor;
          final backgroundColor = backgroundTween.value?.evaluate(curve);

          return builder(context, foregroundColor, backgroundColor);
        },
      ),
    );
  }
}
