import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';

enum _ButtonState { idle, pressed, active }

class ToolbarButton extends HookWidget {
  const ToolbarButton({
    required this.onTap,
    required this.builder,
    this.onTapDown,
    this.prepareMutationOnTapDown = false,
    this.isActive = false,
    this.isDisabled = false,
    this.isRepeatable = false,
    this.color,
    super.key,
  });

  final Widget Function(BuildContext context, Color color, Color? backgroundColor) builder;

  final Color? color;
  final void Function()? onTapDown;
  final bool prepareMutationOnTapDown;
  final bool isActive;
  final bool isDisabled;
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
    final textBrandColor = context.colors.textBrand;
    final borderDefaultColor = context.colors.borderDefault;
    final surfaceDefaultColor = context.colors.surfaceDefault;
    final surfaceMutedColor = context.colors.surfaceMuted;

    useEffect(() {
      foregroundTween.value = ColorTween(
        begin: foregroundTween.value?.evaluate(curve) ?? defaultForegroundColor,
        end: switch (effectiveState) {
          _ButtonState.idle => color ?? textSubtleColor,
          _ButtonState.pressed => borderDefaultColor,
          _ButtonState.active => textBrandColor,
        },
      );

      backgroundTween.value = ColorTween(
        begin: backgroundTween.value?.evaluate(curve),
        end: switch (effectiveState) {
          _ButtonState.idle => surfaceDefaultColor,
          _ButtonState.pressed => surfaceDefaultColor,
          _ButtonState.active => surfaceMutedColor,
        },
      );

      return null;
    }, [effectiveState, textSubtleColor, borderDefaultColor, textBrandColor, surfaceDefaultColor, surfaceMutedColor]);

    useEffect(() {
      unawaited(controller.forward(from: 0));
      return null;
    }, [effectiveState]);

    useEffect(() {
      return repeatTimer.value?.cancel;
    }, []);

    if (isDisabled) {
      return Opacity(opacity: 0.5, child: builder(context, color ?? textSubtleColor, null));
    }

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
      onTapDown: (_) {
        state.value = _ButtonState.pressed;
        if (prepareMutationOnTapDown) {
          NativeEditorToolbarScope.maybeOf(context)?.prepareMutation();
        }
        onTapDown?.call();
      },
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
