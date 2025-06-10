import 'dart:io';

import 'package:collection/collection.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/__generated__/persist_blob_as_image.req.gql.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/buttons/floating.dart';
import 'package:typie/services/blob.dart';

class ImageFloatingToolbar extends HookWidget {
  const ImageFloatingToolbar({super.key});

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
            icon: LucideLightIcons.image,
            onTap: () async {
              final nodeId = proseMirrorState.currentNode!.attrs?['nodeId'] as String?;
              if (nodeId == null) {
                return;
              }

              final result = await FilePicker.platform.pickFiles(type: FileType.image);
              if (result == null) {
                return;
              }

              final pickedFile = result.files.firstOrNull;
              if (pickedFile == null) {
                return;
              }

              final file = File(pickedFile.path!);
              final mimetype = await blob.mime(file);

              final url = file.uri.replace(scheme: 'picker', queryParameters: {'type': mimetype}).toString();

              await scope.webViewController.value?.emitEvent('nodeview', {
                'nodeId': nodeId,
                'name': 'inflight',
                'detail': {'url': url},
              });

              try {
                final path = await blob.upload(file);
                final result = await client.request(
                  GEditorScreen_PersistBlobAsImage_MutationReq((b) => b..vars.input.path = path),
                );

                await scope.webViewController.value?.emitEvent('nodeview', {
                  'nodeId': nodeId,
                  'name': 'success',
                  'detail': {
                    'attrs': {
                      'id': result.persistBlobAsImage.id,
                      'url': result.persistBlobAsImage.url,
                      'ratio': result.persistBlobAsImage.ratio,
                      'placeholder': result.persistBlobAsImage.placeholder,
                      'size': result.persistBlobAsImage.size,
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
