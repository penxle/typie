import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/screens/entity_tree/__generated__/query.req.gql.dart';
import 'package:typie/screens/entity_tree/__generated__/root_query.req.gql.dart';
import 'package:typie/screens/entity_tree/entity_tree.dart';
import 'package:typie/widgets/screen.dart';

@RoutePage()
class EntityTreeScreen extends HookWidget {
  const EntityTreeScreen(@PathParam() this.entityId, {super.key});

  final String? entityId;

  @override
  Widget build(BuildContext context) {
    return Screen(
      child:
          entityId == null
              ? GraphQLOperation(
                operation: GEntityTreeScreen_Root_QueryReq(),
                builder: (context, client, data) {
                  final entities = data.me!.sites[0].entities.toList();

                  return EntityTree(entities);
                },
              )
              : GraphQLOperation(
                operation: GEntityTreeScreen_QueryReq((b) => b..vars.entityId = entityId),
                builder: (context, client, data) {
                  final entities = data.entity.children.toList();

                  return EntityTree(entities);
                },
              ),
    );
  }
}
