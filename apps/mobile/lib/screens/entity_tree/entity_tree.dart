import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/entity_tree/__generated__/entity_tree_entity_entity.data.gql.dart';
import 'package:typie/screens/entity_tree/entity.dart';

class EntityTree extends HookWidget {
  const EntityTree(this.entities, {super.key});

  final List<GEntityTree_Entity_entity> entities;

  @override
  Widget build(BuildContext context) {
    if (entities.isEmpty) {
      return const Center(child: Text('폴더가 비어있어요', style: TextStyle(fontSize: 16, color: Colors.grey)));
    }

    return ListView.builder(
      itemCount: entities.length,
      itemBuilder: (context, index) {
        return ListTile(title: Entity(entities[index]));
      },
    );
  }
}
