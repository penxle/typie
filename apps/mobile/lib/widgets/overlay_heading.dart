import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:typie/context/theme.dart';

class OverlayHeading extends StatelessWidget {
  const OverlayHeading({required this.leading, required this.title, required this.scrollController, super.key});

  static const height = 48.0;
  static const gradientHeight = 24.0;
  static const contentTopSpacing = height + gradientHeight;
  static const revealOffset = 44.0;

  final Widget leading;
  final String title;
  final ScrollController scrollController;

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
        final showTitle = currentOffset > revealOffset;

        return Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Container(
              height: height,
              padding: const Pad(horizontal: 20),
              decoration: BoxDecoration(color: context.colors.surfaceSubtle),
              child: Stack(
                alignment: Alignment.center,
                children: [
                  Align(alignment: Alignment.centerLeft, child: leading),
                  AnimatedSlide(
                    offset: Offset(0, showTitle ? 0.0 : 0.4),
                    duration: const Duration(milliseconds: 200),
                    curve: Curves.easeOut,
                    child: AnimatedOpacity(
                      opacity: showTitle ? 1.0 : 0.0,
                      duration: const Duration(milliseconds: 150),
                      child: Text(title, style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w700)),
                    ),
                  ),
                ],
              ),
            ),
            Container(
              height: gradientHeight,
              decoration: BoxDecoration(
                gradient: LinearGradient(
                  begin: Alignment.topCenter,
                  end: Alignment.bottomCenter,
                  colors: [context.colors.surfaceSubtle, context.colors.surfaceSubtle.withValues(alpha: 0)],
                ),
              ),
            ),
          ],
        );
      },
    );
  }
}
