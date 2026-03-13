import 'dart:async';

import 'package:flutter/material.dart';
import 'package:gap/gap.dart';
import 'package:skeletonizer/skeletonizer.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/widgets/heading.dart';

class OverlayHeading extends StatelessWidget implements PreferredSizeWidget {
  const OverlayHeading({
    required this.leading,
    required this.title,
    required this.scrollController,
    this.trailing,
    this.visible = true,
    this.onTap,
    super.key,
  });

  static const height = 48.0;
  static const gradientHeight = 16.0;
  static const contentTopSpacing = height;
  static const revealOffset = 44.0;
  static const _defaultFadeStops = [0.0, 0.66, 0.82, 1.0];

  static double overlayHeight(BuildContext context) => MediaQuery.viewPaddingOf(context).top + height + gradientHeight;

  static double titleTopPadding(BuildContext context, {double extra = 8}) =>
      MediaQuery.viewPaddingOf(context).top + height + extra;

  static List<Color> buildFadeColors(
    BuildContext context, {
    Color? baseColor,
    double topAlpha = 0.9,
    double secondAlpha = 0.8,
    double thirdAlpha = 0.5,
  }) {
    final color = baseColor ?? context.colors.surfaceSubtle;

    return [
      color.withValues(alpha: topAlpha),
      color.withValues(alpha: secondAlpha),
      color.withValues(alpha: thirdAlpha),
      color.withValues(alpha: 0),
    ];
  }

  static Color controlBackgroundColor(BuildContext context) => switch (context.theme.brightness) {
    Brightness.dark => context.colors.surfaceSubtle,
    Brightness.light => context.colors.surfaceDefault,
  };

  static List<BoxShadow> controlShadow(BuildContext context) => [
    BoxShadow(color: context.colors.shadowDefault.withValues(alpha: 0.06), offset: const Offset(0, 1), blurRadius: 4),
  ];

  final Widget leading;
  final String title;
  final ScrollController scrollController;
  final Widget? trailing;
  final bool visible;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    return AnimatedSlide(
      offset: Offset(0, visible ? 0 : -1),
      duration: const Duration(milliseconds: 200),
      curve: Curves.easeOutCubic,
      child: AnimatedOpacity(
        opacity: visible ? 1 : 0,
        duration: const Duration(milliseconds: 150),
        child: OverlayHeadingBar(
          leading: leading,
          center: OverlayHeadingRevealTitle(scrollController: scrollController, title: title),
          trailing: trailing,
          onTap: onTap,
        ),
      ),
    );
  }

  @override
  Size get preferredSize => const Size.fromHeight(height);
}

class OverlayHeadingLayout extends StatelessWidget {
  const OverlayHeadingLayout({required this.child, this.heading, super.key});

  final Widget child;
  final Widget? heading;

  @override
  Widget build(BuildContext context) {
    final resolvedFadeColors = OverlayHeading.buildFadeColors(context);

    return Stack(
      children: [
        Positioned.fill(child: child),
        Positioned(top: 0, left: 0, right: 0, child: OverlayHeadingFade(colors: resolvedFadeColors)),
        if (heading != null) Positioned(top: 0, left: 0, right: 0, child: heading!),
      ],
    );
  }
}

class OverlayHeadingFade extends StatelessWidget {
  const OverlayHeadingFade({required this.colors, this.stops = OverlayHeading._defaultFadeStops, super.key});

  final List<Color> colors;
  final List<double> stops;

  @override
  Widget build(BuildContext context) {
    return Skeleton.ignore(
      child: IgnorePointer(
        child: SizedBox(
          height: OverlayHeading.overlayHeight(context),
          child: DecoratedBox(
            decoration: BoxDecoration(
              gradient: LinearGradient(
                begin: Alignment.topCenter,
                end: Alignment.bottomCenter,
                colors: colors,
                stops: stops,
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class OverlayHeadingBar extends StatelessWidget implements PreferredSizeWidget {
  const OverlayHeadingBar({
    this.leading,
    this.center,
    this.trailing,
    this.backgroundColor = Colors.transparent,
    this.leadingWidth = HeadingCircleButton.slotWidth,
    this.trailingWidth = HeadingCircleButton.slotWidth,
    this.maxCenterWidth = 420,
    this.onTap,
    super.key,
  });

  final Widget? leading;
  final Widget? center;
  final Widget? trailing;
  final Color backgroundColor;
  final double leadingWidth;
  final double trailingWidth;
  final double? maxCenterWidth;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final constrainedCenter = switch (maxCenterWidth) {
      final double width => ConstrainedBox(
        constraints: BoxConstraints(maxWidth: width),
        child: center ?? const SizedBox.shrink(),
      ),
      null => center ?? const SizedBox.shrink(),
    };

    return AnnotatedRegion(
      value: buildHeadingSystemUiOverlayStyle(context),
      child: Listener(
        onPointerDown: (_) => onTap?.call(),
        child: DecoratedBox(
          decoration: BoxDecoration(color: backgroundColor),
          child: SafeArea(
            bottom: false,
            child: SizedBox(
              height: OverlayHeading.height,
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
  Size get preferredSize => const Size.fromHeight(OverlayHeading.height);
}

class OverlayHeadingBackButton extends StatelessWidget {
  const OverlayHeadingBackButton({this.onTap, this.icon = LucideLightIcons.chevron_left, super.key});

  // ignore: avoid_futureor_void -- matches HeadingCircleButton and allows sync/async handlers
  final FutureOr<void> Function()? onTap;
  final IconData icon;

  @override
  Widget build(BuildContext context) {
    return HeadingCircleButton(
      icon: icon,
      backgroundColor: OverlayHeading.controlBackgroundColor(context),
      boxShadow: OverlayHeading.controlShadow(context),
      useSlotHeight: false,
      onTap: onTap,
    );
  }
}

class OverlayHeadingRevealTitle extends StatelessWidget {
  const OverlayHeadingRevealTitle({
    required this.scrollController,
    required this.title,
    this.style = const TextStyle(fontSize: 16, fontWeight: FontWeight.w700),
    super.key,
  });

  final ScrollController scrollController;
  final String title;
  final TextStyle style;

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: scrollController,
      builder: (context, _) {
        var currentOffset = 0.0;
        for (final position in scrollController.positions) {
          if (position.pixels > currentOffset) {
            currentOffset = position.pixels;
          }
        }
        final showTitle = currentOffset > OverlayHeading.revealOffset;

        return AnimatedSlide(
          offset: Offset(0, showTitle ? 0 : 0.4),
          duration: const Duration(milliseconds: 200),
          curve: Curves.easeOut,
          child: AnimatedOpacity(
            opacity: showTitle ? 1.0 : 0.0,
            duration: const Duration(milliseconds: 150),
            child: Text(title, style: style, maxLines: 1, overflow: TextOverflow.ellipsis),
          ),
        );
      },
    );
  }
}
