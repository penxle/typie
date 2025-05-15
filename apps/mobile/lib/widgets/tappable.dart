import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';

class Tappable extends HookWidget {
  const Tappable({required this.onTap, this.child, this.builder, super.key})
    : assert(child != null || builder != null, 'Either child or builder must be provided');

  final Widget? child;
  final Widget Function(BuildContext context, {bool isPressed})? builder;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    final isPressed = useState(false);

    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: onTap,
      onTapDown: (_) => isPressed.value = true,
      onTapUp: (_) => isPressed.value = false,
      onTapCancel: () => isPressed.value = false,
      child: builder != null ? builder!(context, isPressed: isPressed.value) : child,
    );
  }
}

class AnimatedTappable extends HookWidget {
  const AnimatedTappable({
    required this.builder,
    required this.onTap,
    this.duration = const Duration(milliseconds: 150),
    super.key,
  });

  final Widget Function(BuildContext context, Animation<double> animation) builder;
  final Duration duration;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    final animationController = useAnimationController(duration: duration);

    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: onTap,
      onTapDown: (_) => animationController.forward(),
      onTapUp: (_) => animationController.reverse(),
      onTapCancel: animationController.reverse,
      child: builder(context, animationController),
    );
  }
}
