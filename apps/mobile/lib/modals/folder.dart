import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/full_screen_modal.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/modals/__generated__/folder_create_folder_mutation.req.gql.dart';
import 'package:typie/modals/__generated__/folder_create_post_mutation.req.gql.dart';
import 'package:typie/modals/__generated__/folder_delete_folder_mutation.req.gql.dart';
import 'package:typie/modals/move_entity.dart';
import 'package:typie/screens/entity_tree/__generated__/entity_tree_folder_folder.data.gql.dart';
import 'package:typie/widgets/btn.dart';
import 'package:url_launcher/url_launcher.dart';

class FolderModal extends HookWidget {
  const FolderModal(this.folder, {required this.siteId, super.key});

  final GEntityTree_Folder_folder folder;
  final String siteId;

  @override
  Widget build(BuildContext context) {
    final client = useService<GraphQLClient>();

    const maxDepth = 3;

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
        Btn('이름 변경', onTap: () async {}),
        Btn(
          '사이트에서 열기',
          onTap: () async {
            await launchUrl(Uri.parse(folder.entity.url), mode: LaunchMode.externalApplication);
          },
        ),
        Btn('공유', onTap: () {}),
        Btn(
          '하위 포스트 생성',
          onTap: () async {
            await client.request(
              GFolder_CreatePost_MutationReq(
                (b) =>
                    b
                      ..vars.input.siteId = siteId
                      ..vars.input.parentEntityId = folder.entity.id,
              ),
            );
          },
        ),
        if (!(folder.entity.depth + 1 >= maxDepth))
          Btn(
            '하위 폴더 생성',
            onTap: () async {
              await client.request(
                GFolder_CreateFolder_MutationReq(
                  (b) =>
                      b
                        ..vars.input.siteId = siteId
                        ..vars.input.parentEntityId = folder.entity.id
                        ..vars.input.name = '새 폴더',
                ),
              );
            },
          ),
        Btn(
          '삭제',
          onTap: () async {
            await client.request(GFolder_DeleteFolder_MutationReq((b) => b..vars.input.folderId = folder.id));
          },
        ),
      ],
    );
  }
}
