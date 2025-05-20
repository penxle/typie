import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/icons/lucide.dart';
import 'package:typie/modals/post.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/entity_tree/__generated__/entity_tree_post_post.data.gql.dart';
import 'package:typie/widgets/tappable.dart';

class Post extends HookWidget {
  const Post(this.post, {super.key});

  final GEntityTree_Post_post post;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      child: Row(
        spacing: 8,
        children: [
          const Icon(LucideIcons.file, size: 18),
          Expanded(child: Text(post.title, overflow: TextOverflow.ellipsis, maxLines: 1)),
          Tappable(
            child: const Icon(LucideIcons.ellipsis_vertical, size: 18),
            onTap: () async {
              await context.showBottomSheet(PostModal(post: post));
            },
          ),
        ],
      ),
      onTap: () async {
        await context.router.push(EditorRoute(slug: post.entity.slug));
      },
    );
  }
}
