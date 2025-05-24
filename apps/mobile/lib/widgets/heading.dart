import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:gap/gap.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/tappable.dart';
import 'package:typie/widgets/vertical_divider.dart';

class Heading extends StatelessWidget implements PreferredSizeWidget {
  const Heading({
    this.title,
    this.titleWidget,
    this.actions,
    this.backgroundColor = AppColors.gray_50,
    this.fallbackSystemUiOverlayStyle,
    super.key,
  }) : assert(title != null || titleWidget != null, 'title or titleWidget must be provided');

  final String? title;
  final Widget? titleWidget;
  final List<Widget>? actions;
  final Color? backgroundColor;
  final SystemUiOverlayStyle? fallbackSystemUiOverlayStyle;

  static const _preferredSize = Size.fromHeight(52);

  @override
  Widget build(BuildContext context) {
    return AnnotatedRegion(
      key: context.router.current.key,
      value: const SystemUiOverlayStyle(
        statusBarBrightness: Brightness.light,
        statusBarIconBrightness: Brightness.dark,
        systemNavigationBarDividerColor: AppColors.transparent,
        systemNavigationBarColor: AppColors.transparent,
        systemNavigationBarIconBrightness: Brightness.dark,
        systemNavigationBarContrastEnforced: false,
        systemStatusBarContrastEnforced: false,
      ),
      child: DecoratedBox(
        decoration: BoxDecoration(color: backgroundColor),
        child: SafeArea(
          bottom: false,
          child: Container(
            height: _preferredSize.height,
            margin: const Pad(horizontal: 20),
            decoration: const BoxDecoration(
              border: Border.symmetric(horizontal: BorderSide(color: AppColors.gray_950)),
            ),
            child: Row(
              children: [
                if (ModalRoute.canPopOf(context) ?? false) ...[
                  Tappable(
                    onTap: () => context.router.maybePop(),
                    child: const SizedBox(
                      width: 52,
                      child: Icon(LucideLightIcons.chevron_left, color: AppColors.gray_950),
                    ),
                  ),
                  const AppVerticalDivider(color: AppColors.gray_950),
                  const Gap(20),
                ],
                Expanded(
                  child:
                      titleWidget ??
                      Text(
                        title!,
                        style: const TextStyle(fontSize: 20, fontWeight: FontWeight.w700),
                        overflow: TextOverflow.ellipsis,
                      ),
                ),
                if (actions != null) ...[
                  const Gap(20),
                  const AppVerticalDivider(color: AppColors.gray_950),
                  ...actions!,
                ],
              ],
            ),
          ),
        ),
      ),
    );
  }

  @override
  Size get preferredSize => _preferredSize;
}

class EmptyHeading extends StatelessWidget implements PreferredSizeWidget {
  const EmptyHeading({super.key, this.backgroundColor, this.fallbackSystemUiOverlayStyle});

  final Color? backgroundColor;
  final SystemUiOverlayStyle? fallbackSystemUiOverlayStyle;

  @override
  Size get preferredSize => Size.zero;

  @override
  Widget build(BuildContext context) {
    const child = SafeArea(child: SizedBox.shrink());

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
      child: backgroundColor == null ? child : ColoredBox(color: backgroundColor!, child: child),
    );
  }
}

class HeadingAction extends StatelessWidget {
  const HeadingAction({required this.icon, required this.onTap, super.key});

  final IconData icon;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      onTap: onTap,
      padding: const Pad(vertical: 4),
      child: ConstrainedBox(constraints: const BoxConstraints(minWidth: 52), child: Icon(icon, size: 24)),
    );
  }
}
