import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/screens/entity_tree/__generated__/entity_tree_entity_entity.data.gql.dart';
import 'package:typie/screens/entity_tree/__generated__/entity_tree_move_entity_mutation.req.gql.dart';
import 'package:typie/screens/entity_tree/entity.dart';

class EntityTree extends HookWidget {
  const EntityTree(this.entities, {super.key});

  final List<GEntityTree_Entity_entity> entities;

  @override
  Widget build(BuildContext context) {
    final client = useService<GraphQLClient>();

    if (entities.isEmpty) {
      return const Center(child: Text('폴더가 비어있어요', style: TextStyle(fontSize: 16, color: Colors.grey)));
    }

    return ReorderableListView.builder(
      itemCount: entities.length,
      itemBuilder: (context, index) {
        return ListTile(key: ValueKey(entities[index].id), title: Entity(entities[index]));
      },
      onReorder: (oldIndex, newIndex) async {
        String? lowerOrder;
        String? upperOrder;

        if (newIndex >= entities.length) {
          lowerOrder = entities[entities.length - 1].order;
        } else if (newIndex == 0) {
          upperOrder = entities[0].order;
        } else {
          lowerOrder = entities[newIndex - 1].order;
          upperOrder = entities[newIndex].order;
        }

        await client.request(
          GEntityTree_MoveEntity_MutationReq(
            (b) =>
                b
                  ..vars.input.entityId = entities[oldIndex].id
                  ..vars.input.lowerOrder = lowerOrder
                  ..vars.input.upperOrder = upperOrder,
          ),
        );
      },
      proxyDecorator: (child, index, animation) {
        return Material(
          color: Colors.transparent,
          child: Container(
            decoration: BoxDecoration(color: Colors.grey.shade200, borderRadius: BorderRadius.circular(8)),
            child: child,
          ),
        );
      },
    );
  }
}
