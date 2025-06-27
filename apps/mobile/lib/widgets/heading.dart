import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/tappable.dart';
import 'package:typie/widgets/vertical_divider.dart';

class Heading extends StatelessWidget implements PreferredSizeWidget {
  const Heading({
    this.leadingWidget,
    this.titleIcon,
    this.title,
    this.titleWidget,
    this.actions,
    this.backgroundColor,
    this.fallbackSystemUiOverlayStyle,
    super.key,
  }) : assert(title != null || titleWidget != null, 'title or titleWidget must be provided');

  final Widget? leadingWidget;
  final IconData? titleIcon;
  final String? title;
  final Widget? titleWidget;
  final List<Widget>? actions;
  final Color? backgroundColor;
  final SystemUiOverlayStyle? fallbackSystemUiOverlayStyle;

  static const _preferredSize = Size.fromHeight(52);

  @override
  Widget build(BuildContext context) {
    final route = ModalRoute.of(context);

    return AnnotatedRegion(
      key: context.router.current.key,
      value: SystemUiOverlayStyle(
        statusBarColor: AppColors.transparent,
        statusBarBrightness: Theme.of(context).brightness == Brightness.light ? Brightness.light : Brightness.dark,
        statusBarIconBrightness: Theme.of(context).brightness == Brightness.light ? Brightness.dark : Brightness.light,
        systemNavigationBarDividerColor: AppColors.transparent,
        systemNavigationBarColor: AppColors.transparent,
        systemNavigationBarIconBrightness: Theme.of(context).brightness == Brightness.light ? Brightness.dark : Brightness.light,
        systemNavigationBarContrastEnforced: false,
        systemStatusBarContrastEnforced: false,
      ),
      child: DecoratedBox(
        decoration: BoxDecoration(color: backgroundColor ?? context.colors.surfaceSubtle),
        child: SafeArea(
          bottom: false,
          child: Container(
            height: _preferredSize.height,
            margin: const Pad(horizontal: 20),
            decoration: BoxDecoration(
              border: Border.symmetric(horizontal: BorderSide(color: context.colors.borderModal)),
            ),
            child: Row(
              children: [
                if (leadingWidget != null) ...[
                  leadingWidget!,
                  AppVerticalDivider(color: context.colors.borderModal),
                  const Gap(20),
                ] else if (route?.canPop ?? false) ...[
                  Tappable(
                    onTap: () => context.router.maybePop(),
                    padding: const Pad(vertical: 4),
                    child: SizedBox(
                      width: 52,
                      child: Icon(
                        route?.settings is AutoRoutePage && (route!.settings as AutoRoutePage).fullscreenDialog
                            ? LucideLightIcons.x
                            : LucideLightIcons.chevron_left,
                        color: context.colors.textDefault,
                      ),
                    ),
                  ),
                  AppVerticalDivider(color: context.colors.borderModal),
                  const Gap(20),
                ],
                if (titleIcon != null) ...[Icon(titleIcon, size: 20), const Gap(8)],
                Expanded(
                  child:
                      titleWidget ??
                      Text(
                        title!,
                        style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
                        overflow: TextOverflow.ellipsis,
                      ),
                ),
                if (actions != null) ...[
                  const Gap(20),
                  AppVerticalDivider(color: context.colors.borderModal),
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

class HeadingLeading extends StatelessWidget {
  const HeadingLeading({required this.icon, required this.onTap, super.key});

  final IconData icon;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      onTap: onTap,
      padding: const Pad(vertical: 4),
      child: SizedBox(width: 52, child: Icon(icon, size: 24)),
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
