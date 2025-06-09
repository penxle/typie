import 'package:flutter/material.dart';

class ResponsiveContainer extends StatelessWidget {
  const ResponsiveContainer({required this.child, super.key, this.maxWidth, this.alignment = Alignment.topCenter});
  static const _responsiveBreakpoint = 600.0;

  final Widget child;
  final double? maxWidth;
  final Alignment alignment;

  @override
  Widget build(BuildContext context) {
    final screenWidth = MediaQuery.sizeOf(context).width;
    final effectiveMaxWidth = maxWidth ?? _responsiveBreakpoint;

    if (screenWidth < _responsiveBreakpoint) {
      return child;
    }

    return Align(
      alignment: alignment,
      child: ConstrainedBox(
        constraints: BoxConstraints(maxWidth: effectiveMaxWidth),
        child: child,
      ),
    );
  }
}
