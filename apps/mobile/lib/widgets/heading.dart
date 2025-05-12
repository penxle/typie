import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:typie/icons/lucide.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/tappable.dart';

class Heading extends StatelessWidget implements PreferredSizeWidget {
  const Heading({
    super.key,
    this.leading,
    this.title,
    this.titleWidget,
    this.actions,
    this.backgroundColor = AppColors.white,
    this.fallbackSystemUiOverlayStyle,
    this.titleOnLeft = false,
    this.bottomBorder = true,
  });

  final Widget? leading;
  final String? title;
  final Widget? titleWidget;
  final List<Widget>? actions;
  final Color? backgroundColor;
  final SystemUiOverlayStyle? fallbackSystemUiOverlayStyle;
  final bool titleOnLeft;
  final bool bottomBorder;

  static const _preferredSize = Size.fromHeight(54);

  @override
  Widget build(BuildContext context) {
    return AnnotatedRegion(
      value: const SystemUiOverlayStyle(
        statusBarBrightness: Brightness.light,
        statusBarIconBrightness: Brightness.dark,
        systemNavigationBarDividerColor: AppColors.transparent,
        systemNavigationBarColor: AppColors.transparent,
        systemNavigationBarIconBrightness: Brightness.dark,
        systemNavigationBarContrastEnforced: false,
        systemStatusBarContrastEnforced: false,
      ),
      child: Box(
        decoration: BoxDecoration(color: backgroundColor),
        child: SafeArea(
          bottom: false,
          child: Box(
            height: _preferredSize.height,
            padding: const Pad(horizontal: 20),
            decoration: BoxDecoration(
              border: Border(bottom: BorderSide(color: bottomBorder ? AppColors.gray_100 : Colors.transparent)),
            ),
            child: NavigationToolbar(
              leading: leading ?? const HeadingAutoLeading(),
              middle:
                  titleWidget ??
                  (title == null
                      ? null
                      : Text(title!, style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w700))),
              centerMiddle: !titleOnLeft,
              trailing: actions == null ? null : Row(mainAxisSize: MainAxisSize.min, children: actions!),
            ),
          ),
        ),
      ),
    );
  }

  @override
  Size get preferredSize => _preferredSize;

  static PreferredSizeWidget animated({
    required AnimationController animation,
    required Heading Function(BuildContext context) builder,
  }) {
    return PreferredSize(
      preferredSize: _preferredSize,
      child: AnimatedBuilder(animation: animation, builder: (context, child) => builder(context)),
    );
  }
}

class EmptyHeading extends StatelessWidget implements PreferredSizeWidget {
  const EmptyHeading({super.key, this.backgroundColor, this.fallbackSystemUiOverlayStyle});

  final Color? backgroundColor;
  final SystemUiOverlayStyle? fallbackSystemUiOverlayStyle;

  @override
  Size get preferredSize => Size.zero;

  @override
  Widget build(BuildContext context) {
    return AnnotatedRegion(
      value: const SystemUiOverlayStyle(
        statusBarBrightness: Brightness.light,
        statusBarIconBrightness: Brightness.dark,
        systemNavigationBarDividerColor: AppColors.transparent,
        systemNavigationBarColor: AppColors.transparent,
        systemNavigationBarIconBrightness: Brightness.dark,
        systemNavigationBarContrastEnforced: false,
        systemStatusBarContrastEnforced: false,
      ),
      child: Box(color: backgroundColor, child: const SafeArea(child: SizedBox.shrink())),
    );
  }
}

class HeadingAutoLeading extends StatelessWidget {
  const HeadingAutoLeading({super.key, this.color = AppColors.gray_950});

  final Color? color;

  @override
  Widget build(BuildContext context) {
    return AutoLeadingButton(
      builder: (context, leadingType, action) {
        if (leadingType.isNoLeading) {
          return const SizedBox.shrink();
        }

        return Tappable(
          child: Icon(switch (leadingType) {
            LeadingType.back => LucideIcons.chevron_left,
            LeadingType.close => LucideIcons.x,
            _ => throw UnimplementedError(),
          }, color: color),
          onTap: () => action?.call(),
        );
      },
    );
  }
}
