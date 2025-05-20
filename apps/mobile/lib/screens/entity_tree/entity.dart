import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/entity_tree/__generated__/entity_tree_entity_entity.data.gql.dart';
import 'package:typie/screens/entity_tree/__generated__/entity_tree_folder_folder.data.gql.dart';
import 'package:typie/screens/entity_tree/__generated__/entity_tree_post_post.data.gql.dart';
import 'package:typie/screens/entity_tree/folder.dart';
import 'package:typie/screens/entity_tree/post.dart';

class Entity extends HookWidget {
  const Entity(this.entity, {super.key});

  final GEntityTree_Entity_entity entity;

  @override
  Widget build(BuildContext context) {
    return entity.node.G__typename == 'Folder'
        ? Folder(entity.node as GEntityTree_Folder_folder, entityId: entity.id, siteId: entity.site.id)
        : Post(entity.node as GEntityTree_Post_post);
  }
}
