import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/buttons/label.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/vertical_divider.dart';

class NodeToolbar extends HookWidget {
  const NodeToolbar({this.label, required this.children, this.withDelete = true, super.key});

  final String? label;
  final List<Widget> children;
  final bool withDelete;

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final webViewController = useValueListenable(scope.webViewController);

    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      physics: const AlwaysScrollableScrollPhysics(),
      padding: const Pad(left: 16),
      child: Row(
        spacing: 8,
        children: [
          if (label != null) ...[
            Text(label!, style: const TextStyle(fontSize: 16, color: AppColors.gray_700)),
            const Gap(0),
            const AppVerticalDivider(height: 20),
            const Gap(0),
          ],
          ...children,
          if (withDelete)
            LabelToolbarButton(
              text: '삭제',
              color: AppColors.red_500,
              onTap: () async {
                await scope.command('delete');
                await webViewController?.requestFocus();
              },
            ),
        ],
      ),
    );
  }
}
