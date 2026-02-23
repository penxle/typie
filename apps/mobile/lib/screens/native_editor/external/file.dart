import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/native_editor/__generated__/persist_blob_as_file.req.gql.dart';
import 'package:typie/screens/native_editor/controller/upload.dart';
import 'package:typie/screens/native_editor/external/models.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/services/blob.dart';

class FileWidget extends HookWidget {
  const FileWidget({required this.element, super.key});

  final ExternalElement element;

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final uploadManager = scope.uploadManager;
    final blob = useService<Blob>();
    final client = useService<GraphQLClient>();

    useListenable(uploadManager);

    final fileData = element.data as FileElementData;

    final localUploadId = uploadManager.localFileUploadIds[element.nodeId];
    final currentUploadId = fileData.uploadId ?? localUploadId;

    final asset = fileData.id != null ? uploadManager.fileAssets[fileData.id] : null;
    final inflight = currentUploadId != null ? uploadManager.inflightFiles[currentUploadId] : null;

    final hasFile = asset != null || inflight != null;
    final isUploading = inflight != null && asset == null;

    final displayName = asset?.name ?? inflight?.name ?? '파일';
    final displaySize = _formatFileSize(asset?.size ?? inflight?.size);

    final processedUploadId = useRef<String?>(null);

    useEffect(() {
      if (currentUploadId != null && currentUploadId != processedUploadId.value && inflight != null && asset == null) {
        processedUploadId.value = currentUploadId;
        unawaited(_processUpload(scope, uploadManager, blob, client, context, currentUploadId, inflight));
      }
      return null;
    }, [currentUploadId, inflight, asset]);

    if (!hasFile) {
      return _buildPlaceholder(context);
    }

    return Center(
      child: Container(
        constraints: const BoxConstraints(maxWidth: 400),
        height: 64,
        decoration: BoxDecoration(
          color: context.colors.surfaceMuted,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(color: context.colors.borderSubtle),
        ),
        child: Stack(
          children: [
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              child: Row(
                children: [
                  Icon(LucideLightIcons.file, size: 20, color: context.colors.textSubtle),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        Text(
                          displayName,
                          style: TextStyle(
                            fontSize: 14,
                            fontWeight: FontWeight.w500,
                            color: context.colors.textDefault,
                          ),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                        if (displaySize != null)
                          Text(
                            displaySize,
                            style: TextStyle(fontSize: 12, color: context.colors.textSubtle),
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                      ],
                    ),
                  ),
                  if (isUploading) ...[
                    const SizedBox(width: 12),
                    SizedBox(
                      width: 20,
                      height: 20,
                      child: CircularProgressIndicator(strokeWidth: 2, color: context.colors.textDisabled),
                    ),
                  ],
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildPlaceholder(BuildContext context) {
    return Container(
      height: 48,
      decoration: BoxDecoration(color: context.colors.surfaceMuted, borderRadius: BorderRadius.circular(4)),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
        child: Row(
          children: [
            Icon(LucideLightIcons.file, size: 20, color: context.colors.textDisabled),
            const SizedBox(width: 12),
            Text('파일', style: TextStyle(fontSize: 14, color: context.colors.textDisabled)),
          ],
        ),
      ),
    );
  }

  Future<void> _processUpload(
    NativeEditorToolbarScope scope,
    UploadManager uploadManager,
    Blob blob,
    GraphQLClient client,
    BuildContext context,
    String uploadId,
    InflightFile inflight,
  ) async {
    try {
      final file = File(inflight.path);
      final path = await blob.upload(file);
      final result = await client.request(
        GNativeEditorScreen_PersistBlobAsFile_MutationReq((b) => b..vars.input.path = path),
      );

      final fileResult = result.persistBlobAsFile;
      final asset = FileAsset(id: fileResult.id, url: fileResult.url, name: fileResult.name, size: fileResult.size);
      uploadManager.completeFileUpload(uploadId: uploadId, nodeId: element.nodeId, asset: asset);

      scope.dispatch({'type': 'setFileId', 'nodeId': element.nodeId, 'fileId': fileResult.id});
    } catch (err) {
      uploadManager.failFileUpload(uploadId: uploadId, nodeId: element.nodeId);
      if (context.mounted) {
        context.toast(ToastType.error, '파일 업로드에 실패했습니다');
      }
    }
  }

  String? _formatFileSize(int? size) {
    if (size == null || size <= 0) {
      return null;
    }

    const units = ['B', 'KB', 'MB', 'GB'];
    var unitIndex = 0;
    var fileSize = size.toDouble();

    while (fileSize >= 1024 && unitIndex < units.length - 1) {
      fileSize /= 1024;
      unitIndex++;
    }

    if (unitIndex == 0) {
      return '${fileSize.toInt()} ${units[unitIndex]}';
    }
    return '${fileSize.toStringAsFixed(1)} ${units[unitIndex]}';
  }
}
