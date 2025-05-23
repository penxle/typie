import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';

class Tappable extends StatelessWidget {
  const Tappable({required this.onTap, required this.child, this.padding, this.debugTapArea = false, super.key});

  final Widget child;
  final EdgeInsetsGeometry? padding;
  final void Function() onTap;

  final bool debugTapArea;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: onTap,
      // ignore: deprecated_member_use for debugging
      child: debugTapArea ? Box.rand(padding: padding, child: child) : Box(padding: padding, child: child),
    );
  }
}
