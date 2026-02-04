import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/toolbar/buttons/floating.dart';
import 'package:typie/screens/native_editor/controller/upload.dart';
import 'package:typie/screens/native_editor/external/models.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:uuid/uuid.dart';

class NativeEditorFileFloatingToolbar extends StatelessWidget {
  const NativeEditorFileFloatingToolbar({required this.element, super.key});

  final ExternalElement element;

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final uploadManager = scope.uploadManager;
    final fileData = element.data as FileElementData;

    final localUploadId = uploadManager.localFileUploadIds[element.nodeId];
    final currentUploadId = fileData.uploadId ?? localUploadId;
    final hasFile = fileData.id != null || currentUploadId != null;

    final asset = fileData.id != null ? uploadManager.fileAssets[fileData.id] : null;

    return Row(
      spacing: 8,
      children: [
        if (!hasFile)
          FloatingToolbarButton(
            icon: LucideLightIcons.paperclip,
            onTap: () => _handleUpload(scope, uploadManager, context),
          ),
        if (asset != null)
          FloatingToolbarButton(
            icon: LucideLightIcons.download,
            onTap: () async {
              final uri = Uri.parse(asset.url);
              await launchUrl(uri, mode: LaunchMode.externalApplication);
            },
          ),
        FloatingToolbarButton(
          icon: LucideLightIcons.trash_2,
          onTap: () {
            uploadManager.removeLocalFileUploadId(element.nodeId);
            scope.dispatch({'type': 'deleteNode', 'nodeId': element.nodeId});
          },
        ),
      ],
    );
  }

  Future<void> _handleUpload(NativeEditorToolbarScope scope, UploadManager uploadManager, BuildContext context) async {
    final result = await FilePicker.platform.pickFiles(allowMultiple: true).catchError((err) {
      if (context.mounted) {
        context.toast(ToastType.error, '파일을 선택할 수 없습니다');
      }
      return null;
    });

    if (result == null || result.files.isEmpty) {
      return;
    }

    final platformFiles = result.files.where((f) => f.path != null).toList();
    if (platformFiles.isEmpty) {
      return;
    }

    const uuid = Uuid();
    final fileData = element.data as FileElementData;

    final firstFile = platformFiles.first;
    final firstUploadId = fileData.uploadId ?? uuid.v4();

    if (fileData.uploadId == null) {
      uploadManager.setLocalFileUploadId(element.nodeId, firstUploadId);
    }

    uploadManager.addInflightFile(
      firstUploadId,
      InflightFile(path: firstFile.path!, name: firstFile.name, size: firstFile.size),
    );

    final restFiles = platformFiles.sublist(1);
    for (final platformFile in restFiles) {
      final uploadId = uuid.v4();
      uploadManager.addInflightFile(
        uploadId,
        InflightFile(path: platformFile.path!, name: platformFile.name, size: platformFile.size),
      );
      scope.dispatch({'type': 'insertFile', 'uploadId': uploadId});
    }
  }
}
