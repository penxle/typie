import 'package:flutter/material.dart';

class HorizontalDivider extends StatelessWidget {
  const HorizontalDivider({super.key, this.height = 1.0, required this.color});

  final double height;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Divider(height: height, color: color);
  }
}
