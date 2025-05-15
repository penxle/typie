import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:typie/styles/colors.dart';

class Screen extends StatelessWidget {
  const Screen({
    required this.child,
    super.key,
    this.actions,
    this.safeArea = true,
    this.heading,
    this.resizeToAvoidBottomInset = true,
    this.keyboardDismiss = true,
    this.padding,
    this.backgroundColor = AppColors.white,
  });

  final PreferredSizeWidget? heading;
  final Widget child;
  final List<Widget>? actions;
  final bool safeArea;
  final bool resizeToAvoidBottomInset;
  final bool keyboardDismiss;
  final EdgeInsets? padding;
  final Color backgroundColor;

  @override
  Widget build(BuildContext context) {
    Widget body = Box(padding: padding, child: child);

    if (keyboardDismiss) {
      body = KeyboardDismiss(child: body);
    }

    if (safeArea) {
      body = SafeArea(maintainBottomViewPadding: !resizeToAvoidBottomInset, child: body);
    }

    return Scaffold(
      resizeToAvoidBottomInset: resizeToAvoidBottomInset,
      backgroundColor: backgroundColor,
      appBar: heading,
      body: body,
    );
  }
}
