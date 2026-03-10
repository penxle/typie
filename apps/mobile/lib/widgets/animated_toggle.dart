import 'package:flutter/widgets.dart';

/// Boolean 상태 변화만 애니메이션하고, 테마 등 외부 값은 즉시 반영하는 위젯.
///
/// [AnimatedContainer]와 달리, builder에서 읽는 테마 컬러 등은
/// 애니메이션 대상이 아니므로 테마 전환 시 다른 요소와 동시에 바뀐다.
class AnimatedToggle extends StatelessWidget {
  const AnimatedToggle({
    super.key,
    required this.value,
    required this.builder,
    this.duration = const Duration(milliseconds: 200),
    this.curve = Curves.easeInOut,
    this.child,
  });

  final bool value;
  final Duration duration;
  final Curve curve;
  final Widget Function(BuildContext context, double t, Widget? child) builder;
  final Widget? child;

  @override
  Widget build(BuildContext context) {
    return TweenAnimationBuilder<double>(
      tween: Tween(end: value ? 1.0 : 0.0),
      duration: duration,
      curve: curve,
      builder: builder,
      child: child,
    );
  }
}
