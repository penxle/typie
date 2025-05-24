import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/tappable.dart';

extension BottomMenuExtension on BuildContext {
  Future<T?> showBottomMenu<T extends Object?>({required List<BottomMenuItem> items}) {
    return this.showBottomSheet(_Widget(items: items));
  }
}

class _Widget extends StatelessWidget {
  const _Widget({required this.items});

  final List<BottomMenuItem> items;

  @override
  Widget build(BuildContext context) {
    return Column(children: items);
  }
}

class BottomMenuItem extends StatelessWidget {
  const BottomMenuItem({required this.icon, required this.label, required this.onTap, super.key});

  final IconData icon;
  final String label;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      padding: const Pad(horizontal: 20, vertical: 12),
      onTap: () {
        context.router.pop();
        onTap();
      },
      child: Row(
        spacing: 12,
        children: [
          Icon(icon, size: 24, color: AppColors.gray_700),
          Text(label, style: const TextStyle(fontSize: 17, fontWeight: FontWeight.w500)),
        ],
      ),
    );
  }
}
