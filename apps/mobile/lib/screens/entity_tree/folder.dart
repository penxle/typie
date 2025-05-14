import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/icons/lucide.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/entity_tree/__generated__/entity_tree_folder_folder.data.gql.dart';
import 'package:typie/widgets/tappable.dart';

class Folder extends HookWidget {
  const Folder(this.folder, {required this.entityId, super.key});

  final GEntityTree_Folder_folder folder;
  final String entityId;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      child: Row(spacing: 8, children: [const Icon(LucideIcons.folder, size: 18), Text(folder.name)]),
      onTap: () async {
        await context.router.push(EntityTreeRoute(entityId: entityId));
      },
    );
  }
}
