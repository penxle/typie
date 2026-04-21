import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/widgets/overlay_heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';

const settingsCardRadius = 12.0;
const settingsSectionGap = 16.0;
const settingsListRowHeight = 56.0;

class SettingsOverlayScreen extends StatelessWidget {
  const SettingsOverlayScreen({
    required this.title,
    required this.scrollController,
    super.key,
    this.child,
    this.bodyBuilder,
    this.trailing,
    this.leading,
    this.resizeToAvoidBottomInset = false,
    this.bottomAction,
    this.loading = false,
    this.backgroundColor,
    this.padding,
  }) : assert(child != null || bodyBuilder != null),
       assert(child == null || bodyBuilder == null);

  final String title;
  final ScrollController scrollController;
  final Widget? child;
  final Widget Function(BuildContext context, Widget title, EdgeInsets padding)? bodyBuilder;
  final Widget? trailing;
  final Widget? leading;
  final bool resizeToAvoidBottomInset;
  final BottomAction? bottomAction;
  final bool loading;
  final Color? backgroundColor;
  final EdgeInsets? padding;

  @override
  Widget build(BuildContext context) {
    final resolvedPadding =
        padding ??
        EdgeInsets.fromLTRB(20, 0, 20, MediaQuery.paddingOf(context).bottom + (bottomAction == null ? 72 : 96));
    final titleWidget = Padding(
      padding: EdgeInsets.only(top: OverlayHeading.titleTopPadding(context), bottom: 4),
      child: Text(title, style: const TextStyle(fontSize: 24, fontWeight: FontWeight.w800)),
    );
    final body =
        bodyBuilder?.call(context, titleWidget, resolvedPadding) ??
        SingleChildScrollView(
          controller: scrollController,
          physics: const AlwaysScrollableScrollPhysics(),
          padding: resolvedPadding,
          child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [titleWidget, child!]),
        );

    return Screen(
      heading: OverlayHeading(
        title: title,
        scrollController: scrollController,
        leading:
            leading ??
            OverlayHeadingBackButton(
              onTap: () async {
                await context.router.maybePop();
              },
            ),
        trailing: trailing,
      ),
      loading: loading,
      resizeToAvoidBottomInset: resizeToAvoidBottomInset,
      backgroundColor: backgroundColor ?? context.colors.surfaceSubtle,
      bottomAction: bottomAction,
      child: body,
    );
  }
}

class SettingsSectionLabel extends StatelessWidget {
  const SettingsSectionLabel({required this.text, super.key, this.top = 4, this.bottom = 12});

  final String text;
  final double top;
  final double bottom;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: EdgeInsets.only(top: top, bottom: bottom),
      child: Text(
        text,
        style: TextStyle(
          fontSize: 13,
          fontWeight: FontWeight.w700,
          letterSpacing: 0.3,
          color: context.colors.textFaint,
        ),
      ),
    );
  }
}

class SettingsSectionCard extends StatelessWidget {
  const SettingsSectionCard({required this.child, super.key, this.padding, this.clipBehavior = Clip.none});

  final Widget child;
  final EdgeInsets? padding;
  final Clip clipBehavior;

  @override
  Widget build(BuildContext context) {
    return Container(
      decoration: settingsCardDecoration(context),
      clipBehavior: clipBehavior,
      child: Padding(padding: padding ?? EdgeInsets.zero, child: child),
    );
  }
}

class SettingsListRow extends StatelessWidget {
  const SettingsListRow({required this.label, super.key, this.trailing, this.onTap, this.leading, this.dense = false});

  final String label;
  final Widget? trailing;
  final Future<void> Function()? onTap;
  final Widget? leading;
  final bool dense;

  @override
  Widget build(BuildContext context) {
    final child = SizedBox(
      height: dense ? null : settingsListRowHeight,
      child: Row(
        children: [
          if (leading != null) ...[leading!, const SizedBox(width: 12)],
          Expanded(
            child: Text(label, style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w500)),
          ),
          trailing ??
              (onTap != null
                  ? Icon(LucideLightIcons.chevron_right, size: 16, color: context.colors.textFaint)
                  : const SizedBox.shrink()),
        ],
      ),
    );

    if (onTap == null) {
      return Padding(padding: const Pad(horizontal: 16), child: child);
    }

    return Tappable(
      onTap: onTap!,
      padding: const Pad(horizontal: 16),
      child: Tappable.scale(child: child),
    );
  }
}

BoxDecoration settingsCardDecoration(BuildContext context) =>
    BoxDecoration(color: context.colors.surfaceDefault, borderRadius: BorderRadius.circular(settingsCardRadius));
