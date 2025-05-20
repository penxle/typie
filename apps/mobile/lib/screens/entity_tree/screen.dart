import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/screens/entity_tree/__generated__/entity_tree_folder_folder.data.gql.dart';
import 'package:typie/screens/entity_tree/__generated__/query.req.gql.dart';
import 'package:typie/screens/entity_tree/__generated__/root_query.req.gql.dart';
import 'package:typie/screens/entity_tree/entity_tree.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';

@RoutePage()
class EntityTreeScreen extends HookWidget {
  const EntityTreeScreen(@PathParam() this.entityId, {super.key});

  final String? entityId;

  @override
  Widget build(BuildContext context) {
    return entityId == null
        ? GraphQLOperation(
          operation: GEntityTreeScreen_Root_QueryReq(),
          builder: (context, client, data) {
            final entities = data.me!.sites[0].entities.toList();

            return Screen(heading: const Heading(title: '내 포스트'), child: EntityTree(entities));
          },
        )
        : GraphQLOperation(
          operation: GEntityTreeScreen_QueryReq((b) => b..vars.entityId = entityId),
          builder: (context, client, data) {
            final entities = data.entity.children.toList();

            return Screen(
              heading: Heading(
                titleWidget: Text(
                  data.entity.node.G__typename == 'Folder' ? (data.entity.node as GEntityTree_Folder_folder).name : '',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w700),
                ),
              ),
              child: EntityTree(entities),
            );
          },
        );
  }
}
