import 'package:flutter/material.dart';
import 'package:typie/styles/colors.dart';

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
      child: debugTapArea
          ? Container(color: AppColors.red_500, padding: padding, child: child)
          : padding == null
          ? child
          : Padding(padding: padding!, child: child),
    );
  }
}
