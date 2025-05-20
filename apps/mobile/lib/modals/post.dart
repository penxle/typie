import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/full_screen_modal.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/modals/__generated__/post_delete_post_mutation.req.gql.dart';
import 'package:typie/modals/__generated__/post_duplicate_post_mutation.req.gql.dart';
import 'package:typie/modals/move_entity.dart';
import 'package:typie/screens/entity_tree/__generated__/entity_tree_post_post.data.gql.dart';
import 'package:typie/widgets/btn.dart';
import 'package:url_launcher/url_launcher.dart';

class PostModal extends HookWidget {
  const PostModal(this.post, {super.key});

  final GEntityTree_Post_post post;

  @override
  Widget build(BuildContext context) {
    final client = useService<GraphQLClient>();

    return Column(
      mainAxisSize: MainAxisSize.min,
      spacing: 8,
      children: [
        Btn(
          '다른 폴더로 이동',
          onTap: () async {
            await context.showFullScreenModal(MoveEntityModal(post.entity.id));
          },
        ),
        Btn(
          '사이트에서 열기',
          onTap: () async {
            await launchUrl(Uri.parse(post.entity.url), mode: LaunchMode.externalApplication);
          },
        ),
        Btn('공유', onTap: () {}),
        Btn(
          '복제',
          onTap: () async {
            await client.request(GPost_DuplicatePost_MutationReq((b) => b..vars.input.postId = post.id));
          },
        ),
        Btn(
          '삭제',
          onTap: () async {
            await client.request(GPost_DeletePost_MutationReq((b) => b..vars.input.postId = post.id));
          },
        ),
      ],
    );
  }
}
