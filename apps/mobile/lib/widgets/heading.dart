import 'dart:async';

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

SystemUiOverlayStyle buildHeadingSystemUiOverlayStyle(BuildContext context) {
  return SystemUiOverlayStyle(
    statusBarColor: AppColors.transparent,
    statusBarBrightness: switch (context.theme.brightness) {
      Brightness.light => Brightness.light,
      Brightness.dark => Brightness.dark,
    },
    statusBarIconBrightness: switch (context.theme.brightness) {
      Brightness.light => Brightness.dark,
      Brightness.dark => Brightness.light,
    },
    systemNavigationBarColor: AppColors.transparent,
    systemNavigationBarDividerColor: AppColors.transparent,
    systemNavigationBarIconBrightness: switch (context.theme.brightness) {
      Brightness.light => Brightness.dark,
      Brightness.dark => Brightness.light,
    },
    systemNavigationBarContrastEnforced: false,
    systemStatusBarContrastEnforced: false,
  );
}

Color _headingControlBorderColor(BuildContext context) => context.colors.borderStrong;

class Heading extends StatelessWidget implements PreferredSizeWidget {
  const Heading({
    this.leadingWidget,
    this.titleIcon,
    this.titleIconColor,
    this.title,
    this.titleWidget,
    this.suffix,
    this.actions,
    this.backgroundColor,
    this.fallbackSystemUiOverlayStyle,
    this.banner,
    this.onTap,
    super.key,
  }) : assert(title != null || titleWidget != null, 'title or titleWidget must be provided');

  final Widget? leadingWidget;
  final IconData? titleIcon;
  final Color? titleIconColor;
  final String? title;
  final Widget? titleWidget;
  final Widget? suffix;
  final List<Widget>? actions;
  final Color? backgroundColor;
  final SystemUiOverlayStyle? fallbackSystemUiOverlayStyle;
  final Widget? banner;
  final VoidCallback? onTap;

  static const _headingHeight = 52.0;
  static const _bannerHeight = 32.0;

  @override
  Widget build(BuildContext context) {
    final route = ModalRoute.of(context);

    return AnnotatedRegion(
      key: context.router.current.key,
      value: buildHeadingSystemUiOverlayStyle(context),
      child: Listener(
        onPointerDown: (_) => onTap?.call(),
        child: DecoratedBox(
          decoration: BoxDecoration(color: backgroundColor ?? context.colors.surfaceSubtle),
          child: SafeArea(
            bottom: false,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Container(
                  height: const Size.fromHeight(_headingHeight).height,
                  margin: const Pad(horizontal: 20),
                  decoration: BoxDecoration(
                    border: Border.symmetric(horizontal: BorderSide(color: context.colors.borderStrong)),
                  ),
                  child: Row(
                    children: [
                      if (leadingWidget != null) ...[
                        leadingWidget!,
                        AppVerticalDivider(color: context.colors.borderStrong),
                        const Gap(20),
                      ] else if (route?.canPop ?? false) ...[
                        Tappable(
                          onTap: () => context.router.maybePop(),
                          child: SizedBox(
                            width: 52,
                            height: _headingHeight,
                            child: Center(
                              child: Icon(
                                route?.settings is AutoRoutePage && (route!.settings as AutoRoutePage).fullscreenDialog
                                    ? LucideLightIcons.x
                                    : LucideLightIcons.chevron_left,
                                size: 24,
                                color: context.colors.textDefault,
                              ),
                            ),
                          ),
                        ),
                        AppVerticalDivider(color: context.colors.borderStrong),
                        const Gap(20),
                      ],
                      if (titleIcon != null) ...[Icon(titleIcon, size: 20, color: titleIconColor), const Gap(8)],
                      Expanded(
                        child:
                            titleWidget ??
                            Text(
                              title!,
                              style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
                              overflow: TextOverflow.ellipsis,
                            ),
                      ),
                      if (suffix != null) ...[const Gap(8), suffix!],
                      if (actions != null) ...[
                        const Gap(20),
                        AppVerticalDivider(color: context.colors.borderStrong),
                        ...actions!,
                      ],
                    ],
                  ),
                ),
                AnimatedContainer(
                  duration: const Duration(milliseconds: 200),
                  curve: Curves.easeInOut,
                  height: banner != null ? _bannerHeight : 0,
                  child: banner,
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }

  @override
  Size get preferredSize => const Size.fromHeight(_headingHeight + _bannerHeight);
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
      value: buildHeadingSystemUiOverlayStyle(context),
      child: backgroundColor == null ? child : ColoredBox(color: backgroundColor!, child: child),
    );
  }
}

class CapsuleHeading extends StatelessWidget implements PreferredSizeWidget {
  const CapsuleHeading({
    required this.center,
    this.leading,
    this.trailing,
    this.backgroundColor,
    this.leadingWidth = HeadingCircleButton.slotWidth,
    this.trailingWidth = HeadingCircleButton.slotWidth,
    this.maxCenterWidth = 420,
    this.onTap,
    super.key,
  });

  final Widget center;
  final Widget? leading;
  final Widget? trailing;
  final Color? backgroundColor;
  final double leadingWidth;
  final double trailingWidth;
  final double? maxCenterWidth;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final constrainedCenter = maxCenterWidth == null
        ? center
        : ConstrainedBox(
            constraints: BoxConstraints(maxWidth: maxCenterWidth!),
            child: center,
          );

    return AnnotatedRegion(
      value: buildHeadingSystemUiOverlayStyle(context),
      child: Listener(
        onPointerDown: (_) => onTap?.call(),
        child: DecoratedBox(
          decoration: BoxDecoration(color: backgroundColor ?? context.colors.surfaceDefault),
          child: SafeArea(
            bottom: false,
            child: SizedBox(
              height: Heading._headingHeight,
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 20),
                child: Row(
                  children: [
                    SizedBox(
                      width: leadingWidth,
                      child: Align(alignment: Alignment.centerLeft, child: leading ?? const SizedBox.shrink()),
                    ),
                    const Gap(12),
                    Expanded(child: Center(child: constrainedCenter)),
                    const Gap(12),
                    SizedBox(
                      width: trailingWidth,
                      child: Align(alignment: Alignment.centerRight, child: trailing ?? const SizedBox.shrink()),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  @override
  Size get preferredSize => const Size.fromHeight(Heading._headingHeight);
}

class HeadingCircleButton extends StatelessWidget {
  const HeadingCircleButton({
    required this.icon,
    this.onTap,
    this.backgroundColor,
    this.borderColor,
    this.boxShadow,
    this.iconColor,
    this.size = controlSize,
    this.useSlotHeight = true,
    super.key,
  });

  static const controlSize = 44.0;
  static const slotWidth = controlSize;

  final IconData icon;
  // ignore: avoid_futureor_void -- matches Tappable and allows sync/async handlers
  final FutureOr<void> Function()? onTap;
  final Color? backgroundColor;
  final Color? borderColor;
  final List<BoxShadow>? boxShadow;
  final Color? iconColor;
  final double size;
  final bool useSlotHeight;

  @override
  Widget build(BuildContext context) {
    final control = Container(
      width: size,
      height: size,
      decoration: BoxDecoration(
        color: backgroundColor ?? context.colors.surfaceMuted,
        shape: BoxShape.circle,
        border: Border.all(color: borderColor ?? _headingControlBorderColor(context)),
        boxShadow: boxShadow,
      ),
      child: Center(child: Icon(icon, size: 18, color: iconColor ?? context.colors.textDefault)),
    );

    final child = useSlotHeight
        ? SizedBox(
            width: slotWidth,
            height: Heading._headingHeight,
            child: Center(child: control),
          )
        : SizedBox(width: size, height: size, child: control);

    if (onTap == null) {
      return child;
    }

    return Tappable(onTap: onTap!, child: child);
  }
}

class HeadingCapsuleLabel extends StatelessWidget {
  const HeadingCapsuleLabel({
    required this.title,
    this.subtitle,
    this.icon,
    this.backgroundColor,
    this.borderColor,
    this.boxShadow,
    this.iconColor,
    this.height = HeadingCircleButton.controlSize,
    this.borderRadius = 999,
    super.key,
  });

  final String title;
  final String? subtitle;
  final IconData? icon;
  final Color? backgroundColor;
  final Color? borderColor;
  final List<BoxShadow>? boxShadow;
  final Color? iconColor;
  final double height;
  final double borderRadius;

  @override
  Widget build(BuildContext context) {
    final hasSubtitle = subtitle != null && subtitle!.isNotEmpty;
    final squircleBorderRadius = BorderRadius.circular(borderRadius);

    return SizedBox(
      width: double.infinity,
      height: height,
      child: DecoratedBox(
        decoration: ShapeDecoration(
          color: backgroundColor ?? context.colors.surfaceMuted,
          shadows: boxShadow,
          shape: RoundedSuperellipseBorder(
            borderRadius: squircleBorderRadius,
            side: BorderSide(color: borderColor ?? _headingControlBorderColor(context)),
          ),
        ),
        child: ClipRSuperellipse(
          borderRadius: squircleBorderRadius,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 5),
            child: Row(
              children: [
                if (icon != null) ...[
                  Icon(icon, size: 18, color: iconColor ?? context.colors.textSubtle),
                  const Gap(10),
                ],
                Expanded(
                  child: Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        title,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(
                          fontSize: hasSubtitle ? 14 : 15,
                          fontWeight: FontWeight.w600,
                          height: 1,
                          color: context.colors.textDefault,
                        ),
                      ),
                      if (hasSubtitle) ...[
                        const Gap(1),
                        Text(
                          subtitle!,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: TextStyle(
                            fontSize: 11,
                            fontWeight: FontWeight.w500,
                            height: 1,
                            color: context.colors.textFaint,
                          ),
                        ),
                      ],
                    ],
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
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
      child: SizedBox(
        width: 52,
        height: Heading._headingHeight,
        child: Center(child: Icon(icon, size: 24)),
      ),
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
      child: SizedBox(
        width: 52,
        height: Heading._headingHeight,
        child: Center(child: Icon(icon, size: 24)),
      ),
    );
  }
}

class HeadingBanner extends StatelessWidget {
  const HeadingBanner({required this.text, required this.backgroundColor, super.key});

  final String text;
  final Color backgroundColor;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: double.infinity,
      height: 32,
      padding: const EdgeInsets.symmetric(vertical: 6),
      color: backgroundColor,
      child: Text(
        text,
        textAlign: TextAlign.center,
        style: const TextStyle(color: Colors.white, fontSize: 14, fontWeight: FontWeight.w500),
      ),
    );
  }
}
