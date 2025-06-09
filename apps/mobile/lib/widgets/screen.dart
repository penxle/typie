import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/responsive_container.dart';
import 'package:typie/widgets/tappable.dart';

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
    this.responsive = true,
    this.maxWidth,
    this.bottomAction,
  });

  final PreferredSizeWidget? heading;
  final Widget child;
  final bool expand;
  final bool safeArea;
  final bool resizeToAvoidBottomInset;
  final bool keyboardDismiss;
  final EdgeInsets? padding;
  final Color backgroundColor;
  final bool responsive;
  final double? maxWidth;
  final BottomAction? bottomAction;

  @override
  Widget build(BuildContext context) {
    var body = child;

    if (expand) {
      body = SizedBox.expand(child: body);
    }

    if (responsive && bottomAction == null) {
      body = ResponsiveContainer(maxWidth: maxWidth, child: body);
    }

    if (padding != null) {
      body = Padding(padding: padding!, child: body);
    }

    if (bottomAction != null) {
      body = Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Expanded(
            child: responsive ? ResponsiveContainer(maxWidth: maxWidth, child: body) : body,
          ),
          bottomAction!,
        ],
      );
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

class BottomAction extends StatelessWidget {
  const BottomAction({
    required this.text,
    required this.onTap,
    super.key,
    this.color = AppColors.gray_950,
    this.textColor = AppColors.white,
  });

  final String text;
  final VoidCallback onTap;
  final Color color;
  final Color textColor;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      onTap: onTap,
      child: Container(
        alignment: Alignment.center,
        decoration: BoxDecoration(color: color),
        padding: Pad(vertical: 16, bottom: MediaQuery.paddingOf(context).bottom),
        child: Text(
          text,
          style: TextStyle(fontSize: 16, fontWeight: FontWeight.w700, color: textColor),
        ),
      ),
    );
  }
}
