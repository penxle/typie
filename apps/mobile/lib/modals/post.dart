import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/full_screen_modal.dart';
import 'package:typie/modals/move_entity.dart';
import 'package:typie/screens/entity_tree/__generated__/entity_tree_post_post.data.gql.dart';
import 'package:typie/widgets/btn.dart';

class PostModal extends HookWidget {
  const PostModal({required this.post, super.key});

  final GEntityTree_Post_post post;

  @override
  Widget build(BuildContext context) {
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
      ],
    );
  }
}
