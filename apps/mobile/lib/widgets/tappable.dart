import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';

class Tappable extends HookWidget {
  const Tappable({required this.onTap, required this.child, super.key});

  final Widget child;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(behavior: HitTestBehavior.opaque, onTap: onTap, child: child);
  }
}
