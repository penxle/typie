import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/heading.dart';

class Screen extends StatelessWidget {
  const Screen({
    required this.child,
    super.key,
    this.heading = const EmptyHeading(),
    this.expand = true,
    this.safeArea = false,
    this.resizeToAvoidBottomInset = false,
    this.keyboardDismiss = true,
    this.padding,
    this.backgroundColor = AppColors.gray_50,
  });

  final PreferredSizeWidget? heading;
  final Widget child;
  final bool expand;
  final bool safeArea;
  final bool resizeToAvoidBottomInset;
  final bool keyboardDismiss;
  final EdgeInsets? padding;
  final Color backgroundColor;

  @override
  Widget build(BuildContext context) {
    var body = child;

    if (expand) {
      body = SizedBox.expand(child: body);
    }

    if (padding != null) {
      body = Padding(padding: padding!, child: body);
    }

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
