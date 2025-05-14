import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/icons/lucide.dart';
import 'package:typie/screens/entity_tree/__generated__/entity_tree_post_post.data.gql.dart';

class Post extends HookWidget {
  const Post(this.post, {super.key});

  final GEntityTree_Post_post post;

  @override
  Widget build(BuildContext context) {
    return Row(spacing: 8, children: [const Icon(LucideIcons.file, size: 18), Text(post.title)]);
  }
}
