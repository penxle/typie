import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/tappable.dart';

enum BtnVariant { primary, disabled }

class Btn extends StatelessWidget {
  const Btn(this.text, {required this.onTap, this.variant = BtnVariant.primary, super.key});

  final String text;
  final BtnVariant variant;
  final FutureOr<void> Function() onTap;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      onTap: onTap,
      child: Box(
        height: 48,
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(6),
          color: switch (variant) {
            BtnVariant.primary => AppColors.brand_500,
            BtnVariant.disabled => AppColors.gray_400,
          },
        ),
        child: Center(
          child: Text(
            text,
            style: TextStyle(
              fontSize: 15,
              fontWeight: FontWeight.w700,
              color: switch (variant) {
                BtnVariant.primary => AppColors.white,
                BtnVariant.disabled => AppColors.white,
              },
            ),
          ),
        ),
      ),
    );
  }
}
