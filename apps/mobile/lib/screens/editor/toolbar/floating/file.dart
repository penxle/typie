import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/toast.dart';
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

              final result = await FilePicker.platform.pickFiles(allowMultiple: true).catchError((err) {
                if (context.mounted) {
                  context.toast(ToastType.error, '파일을 선택할 수 없습니다');
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
              final allFileInfo = platformFiles;

              // NOTE: 첫 번째 파일은 현재 노드 업데이트
              final firstFileInfo = allFileInfo.first;
              await scope.webViewController.value?.emitEvent('nodeview', {
                'nodeId': nodeId,
                'name': 'inflight',
                'detail': {'name': firstFileInfo.name, 'size': firstFileInfo.size},
              });

              final fileNodePairs = <(File, String)>[(allFiles.first, nodeId)];
              final restFiles = allFiles.sublist(1);
              final restFileInfo = allFileInfo.sublist(1);

              // NOTE: 두 번째 이후 파일 노드들 삽입
              if (restFiles.isNotEmpty) {
                final insertedNodeIds = await scope.webViewController.value?.callProcedure('insertNodes', {
                  'nodes': List.generate(restFiles.length, (i) => {'type': 'file'}).toList(),
                });

                if (insertedNodeIds != null && insertedNodeIds is List) {
                  for (var i = 0; i < insertedNodeIds.length && i < restFiles.length; i++) {
                    final nodeId = insertedNodeIds[i] as String?;
                    if (nodeId == null) {
                      continue;
                    }
                    fileNodePairs.add((restFiles[i], nodeId));

                    final fileInfo = restFileInfo[i];
                    await scope.webViewController.value?.emitEvent('nodeview', {
                      'nodeId': nodeId,
                      'name': 'inflight',
                      'detail': {'name': fileInfo.name, 'size': fileInfo.size},
                    });
                  }
                }
              }

              var failureCount = 0;

              // NOTE: 모든 파일 업로드
              await Future.wait(
                fileNodePairs.map((pair) async {
                  final (file, targetNodeId) = pair;

                  try {
                    final path = await blob.upload(file);
                    final result = await client.request(
                      GEditorScreen_PersistBlobAsFile_MutationReq((b) => b..vars.input.path = path),
                    );

                    await scope.webViewController.value?.emitEvent('nodeview', {
                      'nodeId': targetNodeId,
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
                final errorMessage = failureCount == fileNodePairs.length ? '파일 업로드에 실패했습니다' : '일부 파일 업로드에 실패했습니다';
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
