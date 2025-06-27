import 'package:flutter/material.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/widgets/tappable.dart';

enum BtnVariant { primary, disabled }

class Btn extends StatelessWidget {
  const Btn(this.text, {required this.onTap, this.variant = BtnVariant.primary, super.key});

  final String text;
  final BtnVariant variant;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      onTap: onTap,
      child: Container(
        height: 48,
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(6),
          color: switch (variant) {
            BtnVariant.primary => context.colors.accentBrandDefault,
            BtnVariant.disabled => context.colors.surfaceDisabled,
          },
        ),
        child: Center(
          child: Text(
            text,
            style: TextStyle(
              fontSize: 15,
              fontWeight: FontWeight.w700,
              color: switch (variant) {
                BtnVariant.primary => context.colors.textOnBrand,
                BtnVariant.disabled => context.colors.textDisabled,
              },
            ),
          ),
        ),
      ),
    );
  }
}
