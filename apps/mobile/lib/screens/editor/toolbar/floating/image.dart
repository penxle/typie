import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/toast.dart';
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

              final result = await FilePicker.platform.pickFiles(type: FileType.image, allowMultiple: true).catchError((
                err,
              ) {
                if (context.mounted) {
                  context.toast(ToastType.error, '이미지를 선택할 수 없습니다');
                }
                return null;
              });

              if (result == null || result.files.isEmpty) {
                return;
              }

              // NOTE: 일부 플랫폼/설정에서 path가 null일 수 있음
              final platformFiles = result.files.where((f) => f.path != null).toList();
              if (platformFiles.isEmpty) {
                return;
              }
              final allFiles = platformFiles.map((f) => File(f.path!)).toList();

              // NOTE: 첫 번째 파일은 현재 노드 업데이트
              final firstFile = allFiles.first;
              final firstMimetype = await blob.mime(firstFile);
              final firstUrl = firstFile.uri
                  .replace(scheme: 'picker', queryParameters: {'type': firstMimetype})
                  .toString();
              await scope.webViewController.value?.emitEvent('nodeview', {
                'nodeId': nodeId,
                'name': 'inflight',
                'detail': {'url': firstUrl},
              });

              final fileNodePairs = <(File, String)>[(allFiles.first, nodeId)];

              // NOTE: 두 번째 이후 이미지 노드들 삽입
              if (allFiles.length > 1) {
                final imageAttributesList = <Map<String, dynamic>>[];
                for (var i = 1; i < allFiles.length; i++) {
                  final file = allFiles[i];
                  final mimetype = await blob.mime(file);
                  final url = file.uri.replace(scheme: 'picker', queryParameters: {'type': mimetype}).toString();
                  imageAttributesList.add({'inflightUrl': url});
                }

                final insertedNodeIds = await scope.webViewController.value?.callProcedure('insertNodes', {
                  'nodes': imageAttributesList.map((attrs) => {'type': 'image', 'attrs': attrs}).toList(),
                });

                if (insertedNodeIds != null && insertedNodeIds is List) {
                  final validNodeIds = insertedNodeIds.whereType<String>().toList();
                  for (var i = 0; i < validNodeIds.length; i++) {
                    if (i + 1 < allFiles.length) {
                      fileNodePairs.add((allFiles[i + 1], validNodeIds[i]));
                    }
                  }
                }
              }

              var failureCount = 0;

              // NOTE: 모든 이미지 업로드
              await Future.wait(
                fileNodePairs.map((pair) async {
                  final (file, targetNodeId) = pair;

                  try {
                    final path = await blob.upload(file);
                    final result = await client.request(
                      GEditorScreen_PersistBlobAsImage_MutationReq((b) => b..vars.input.path = path),
                    );

                    await scope.webViewController.value?.emitEvent('nodeview', {
                      'nodeId': targetNodeId,
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
                  } catch (err) {
                    failureCount++;
                    await scope.webViewController.value?.emitEvent('nodeview', {
                      'nodeId': targetNodeId,
                      'name': 'error',
                    });
                  }
                }),
              );

              if (failureCount > 0 && context.mounted) {
                final errorMessage = failureCount == fileNodePairs.length ? '이미지 업로드에 실패했습니다' : '일부 이미지 업로드에 실패했습니다';
                context.toast(ToastType.error, errorMessage);
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
