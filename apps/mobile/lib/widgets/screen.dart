import 'package:flutter/material.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/heading.dart';

class Screen extends StatelessWidget {
  const Screen({
    required this.child,
    super.key,
    this.title,
    this.actions,
    this.useSafeArea = false,
    this.appBar,
    this.bottomBorder = true,
    this.resizeToAvoidBottomInset = true,
    this.padding,
    this.backgroundColor = AppColors.white,
  });

  final PreferredSizeWidget? appBar;
  final String? title;
  final Widget child;
  final List<Widget>? actions;
  final bool useSafeArea;
  final bool bottomBorder;
  final bool resizeToAvoidBottomInset;
  final EdgeInsets? padding;
  final Color backgroundColor;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      resizeToAvoidBottomInset: resizeToAvoidBottomInset,
      backgroundColor: backgroundColor,
      appBar:
          appBar ??
          Heading(
            title:
                title == null ? null : Text(title!, style: const TextStyle(fontSize: 17, fontWeight: FontWeight.w700)),
            actions: actions,
            bottomBorder: bottomBorder,
          ),
      body: SizedBox.expand(
        child:
            useSafeArea
                ? SafeArea(
                  maintainBottomViewPadding: resizeToAvoidBottomInset,
                  child: Padding(padding: padding ?? EdgeInsets.zero, child: child),
                )
                : Padding(padding: padding ?? EdgeInsets.zero, child: child),
      ),
    );
  }
}
