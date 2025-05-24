import 'package:flutter/material.dart';
import 'package:typie/styles/colors.dart';

class AppVerticalDivider extends StatelessWidget {
  const AppVerticalDivider({
    super.key,
    this.width = 1.0,
    this.height = double.infinity,
    this.color = AppColors.gray_100,
  });

  final double width;
  final double height;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: width,
      height: height,
      child: ColoredBox(color: color),
    );
  }
}
