import 'package:flutter/material.dart';
import 'package:typie/context/theme.dart';

class AppVerticalDivider extends StatelessWidget {
  const AppVerticalDivider({
    super.key,
    this.width = 1.0,
    this.height = double.infinity,
    this.color,
  });

  final double width;
  final double height;
  final Color? color;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: width,
      height: height,
      child: ColoredBox(color: color ?? context.colors.borderSubtle),
    );
  }
}
