import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/entity/__generated__/entity_fragment.data.gql.dart';
import 'package:typie/screens/entity/multi_entities_menu.dart';
import 'package:typie/widgets/tappable.dart';

class SelectedEntitiesBar extends StatelessWidget {
  const SelectedEntitiesBar({
    super.key,
    required this.selectedItems,
    required this.entities,
    required this.onClearSelection,
    required this.onExitSelectionMode,
    required this.isVisible,
  });

  final Set<String> selectedItems;
  final List<GEntityScreen_Entity_entity> entities;
  final VoidCallback onClearSelection;
  final VoidCallback onExitSelectionMode;
  final bool isVisible;

  @override
  Widget build(BuildContext context) {
    return AnimatedPositioned(
      duration: const Duration(milliseconds: 200),
      curve: Curves.easeInOut,
      left: 0,
      right: 0,
      bottom: isVisible ? 20 : 10,
      child: AnimatedOpacity(
        opacity: isVisible ? 1.0 : 0.0,
        duration: const Duration(milliseconds: 200),
        child: Center(
          child: IntrinsicWidth(
            child: Container(
              decoration: BoxDecoration(
                color: context.colors.surfaceDefault,
                border: Border.all(color: context.colors.borderStrong),
                borderRadius: const BorderRadius.all(Radius.circular(12)),
                boxShadow: [
                  BoxShadow(color: Colors.black.withValues(alpha: 0.08), blurRadius: 8, offset: const Offset(0, 2)),
                ],
              ),
              padding: const Pad(vertical: 8, left: 18, right: 12),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(
                    '${selectedItems.length}개 선택됨',
                    style: const TextStyle(fontSize: 15, fontWeight: FontWeight.w500),
                  ),
                  const SizedBox(width: 8),
                  Tappable(
                    onTap: onClearSelection,
                    child: Container(
                      padding: const Pad(all: 6),
                      decoration: BoxDecoration(
                        border: Border.all(color: context.colors.borderStrong),
                        borderRadius: const BorderRadius.all(Radius.circular(6)),
                      ),
                      child: const Icon(LucideLightIcons.x, size: 20),
                    ),
                  ),
                  const SizedBox(width: 8),
                  Container(width: 1, height: 32, color: context.colors.borderStrong),
                  const SizedBox(width: 8),
                  Tappable(
                    onTap: () async {
                      await context.showBottomSheet(
                        child: MultiEntitiesMenu(
                          selectedItems: selectedItems,
                          entities: entities,
                          onExitSelectionMode: onExitSelectionMode,
                          via: 'selected_entities_bar',
                        ),
                      );
                    },
                    child: Container(
                      padding: const Pad(all: 6),
                      decoration: BoxDecoration(
                        border: Border.all(color: context.colors.borderStrong),
                        borderRadius: const BorderRadius.all(Radius.circular(6)),
                      ),
                      child: const Icon(LucideLightIcons.ellipsis_vertical, size: 20),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
