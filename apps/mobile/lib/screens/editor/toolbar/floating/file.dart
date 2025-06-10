import 'dart:io';

import 'package:collection/collection.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/__generated__/persist_blob_as_file.req.gql.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/buttons/floating.dart';
import 'package:typie/services/blob.dart';

class FileFloatingToolbar extends HookWidget {
  const FileFloatingToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final blob = useService<Blob>();
    final client = useService<GraphQLClient>();

    final scope = EditorStateScope.of(context);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);

    return Row(
      spacing: 8,
      children: [
        if (proseMirrorState!.currentNode!.attrs?['id'] == null)
          FloatingToolbarButton(
            icon: LucideLightIcons.paperclip,
            onTap: () async {
              final nodeId = proseMirrorState.currentNode!.attrs?['nodeId'] as String?;
              if (nodeId == null) {
                return;
              }

              final result = await FilePicker.platform.pickFiles();
              if (result == null) {
                return;
              }

              final pickedFile = result.files.firstOrNull;
              if (pickedFile == null) {
                return;
              }

              final file = File(pickedFile.path!);

              await scope.webViewController.value?.emitEvent('nodeview', {
                'nodeId': nodeId,
                'name': 'inflight',
                'detail': {'name': pickedFile.name, 'size': pickedFile.size},
              });

              try {
                final path = await blob.upload(file);
                final result = await client.request(
                  GEditorScreen_PersistBlobAsFile_MutationReq((b) => b..vars.input.path = path),
                );

                await scope.webViewController.value?.emitEvent('nodeview', {
                  'nodeId': nodeId,
                  'name': 'success',
                  'detail': {
                    'attrs': {
                      'id': result.persistBlobAsFile.id,
                      'url': result.persistBlobAsFile.url,
                      'name': result.persistBlobAsFile.name,
                      'size': result.persistBlobAsFile.size,
                    },
                  },
                });
              } catch (_) {
                await scope.webViewController.value?.emitEvent('nodeview', {'nodeId': nodeId, 'name': 'error'});
              }
            },
          ),
        FloatingToolbarButton(
          icon: LucideLightIcons.trash_2,
          onTap: () async {
            await scope.command('delete');
          },
        ),
      ],
    );
  }
}
