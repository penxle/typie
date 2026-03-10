import 'dart:async';

import 'package:flutter/material.dart';
import 'package:typie/context/theme.dart';

class Tappable extends StatelessWidget {
  const Tappable({required this.onTap, required this.child, this.padding, this.debugTapArea = false, super.key});

  final Widget child;
  final EdgeInsetsGeometry? padding;
  // ignore: avoid_futureor_void -- to many consumers
  final FutureOr<void> Function() onTap;

  final bool debugTapArea;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: onTap,
      child: debugTapArea
          ? Container(color: context.colors.accentDanger, padding: padding, child: child)
          : padding == null
          ? child
          : Padding(padding: padding!, child: child),
    );
  }
}
