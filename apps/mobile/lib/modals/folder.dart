import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/full_screen_modal.dart';
import 'package:typie/modals/move_entity.dart';
import 'package:typie/screens/entity_tree/__generated__/entity_tree_folder_folder.data.gql.dart';
import 'package:typie/widgets/btn.dart';

class FolderModal extends HookWidget {
  const FolderModal({required this.folder, super.key});

  final GEntityTree_Folder_folder folder;

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      spacing: 8,
      children: [
        Btn(
          '다른 폴더로 이동',
          onTap: () async {
            await context.showFullScreenModal(
              MoveEntityModal(folder.entity.id, depth: folder.maxDescendantFoldersDepth - folder.entity.depth),
            );
          },
        ),
      ],
    );
  }
}
