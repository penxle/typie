import 'package:flutter/material.dart';

class Tappable extends StatelessWidget {
  const Tappable({required this.child, this.onTap, super.key});

  final Widget child;
  final void Function()? onTap;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(behavior: HitTestBehavior.opaque, onTap: onTap, child: child);
  }
}
