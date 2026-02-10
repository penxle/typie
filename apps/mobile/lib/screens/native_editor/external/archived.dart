import 'package:flutter/material.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';

class ArchivedWidget extends StatelessWidget {
  const ArchivedWidget({super.key});

  @override
  Widget build(BuildContext context) {
    return Container(
      height: 48,
      decoration: BoxDecoration(color: context.colors.surfaceMuted, borderRadius: BorderRadius.circular(4)),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
        child: Row(
          children: [
            Icon(LucideLightIcons.archive, size: 20, color: context.colors.textDisabled),
            const SizedBox(width: 12),
            Text('보관된 블록', style: TextStyle(fontSize: 14, color: context.colors.textDisabled)),
          ],
        ),
      ),
    );
  }
}
