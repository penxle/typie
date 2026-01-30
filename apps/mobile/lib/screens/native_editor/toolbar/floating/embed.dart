import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/toolbar/buttons/floating.dart';
import 'package:typie/screens/native_editor/__generated__/unfurl_embed.req.gql.dart';
import 'package:typie/screens/native_editor/external/models.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/upload_manager.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/text_field.dart';
import 'package:url_launcher/url_launcher.dart';

class NativeEditorEmbedFloatingToolbar extends HookWidget {
  const NativeEditorEmbedFloatingToolbar({required this.element, super.key});

  final ExternalElement element;

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final uploadManager = scope.uploadManager;
    final client = useService<GraphQLClient>();

    useListenable(uploadManager);

    final embedData = element.data as EmbedElementData;
    final asset = embedData.id != null ? uploadManager.embedAssets[embedData.id] : null;
    final hasEmbed = embedData.id != null;

    return Row(
      spacing: 8,
      children: [
        if (!hasEmbed)
          FloatingToolbarButton(
            icon: LucideLightIcons.file_up,
            onTap: () => _handleInsert(context, scope, uploadManager, client),
          ),
        if (asset != null)
          FloatingToolbarButton(
            icon: LucideLightIcons.external_link,
            onTap: () async {
              final uri = Uri.tryParse(asset.url);
              if (uri != null) {
                await launchUrl(uri, mode: LaunchMode.externalApplication);
              }
            },
          ),
        FloatingToolbarButton(
          icon: LucideLightIcons.trash_2,
          onTap: () {
            uploadManager.removeInflightEmbed(element.nodeId);
            scope.dispatch({'type': 'deleteNode', 'nodeId': element.nodeId});
          },
        ),
      ],
    );
  }

  Future<void> _handleInsert(
    BuildContext context,
    NativeEditorToolbarScope scope,
    UploadManager uploadManager,
    GraphQLClient client,
  ) async {
    await context.showModal(
      intercept: true,
      child: HookForm(
        onSubmit: (form) async {
          final embedUrl = form.data['url'] as String?;
          if (embedUrl == null || embedUrl.isEmpty) {
            return;
          }

          final url = RegExp(r'^[^:]+:\/\/').hasMatch(embedUrl) ? embedUrl : 'https://$embedUrl';

          uploadManager.setInflightEmbed(element.nodeId, inflight: true);

          try {
            final result = await client.request(
              GNativeEditorScreen_UnfurlEmbed_MutationReq((b) => b..vars.input.url = url),
            );

            final embed = result.unfurlEmbed;
            final asset = EmbedAsset(
              id: embed.id,
              url: embed.url,
              title: embed.title,
              description: embed.description,
              thumbnailUrl: embed.thumbnailUrl,
              html: embed.html,
            );

            uploadManager.completeEmbedUnfurl(nodeId: element.nodeId, asset: asset);
            scope.dispatch({'type': 'setEmbedId', 'nodeId': element.nodeId, 'embedId': embed.id});
          } catch (err) {
            uploadManager.failEmbedUnfurl(nodeId: element.nodeId);
            if (context.mounted) {
              context.toast(ToastType.error, '링크를 임베드할 수 없습니다');
            }
          }
        },
        builder: (context, form) {
          return ConfirmModal(
            title: '임베드 삽입',
            confirmText: '삽입',
            onConfirm: () async {
              await form.submit();
            },
            child: const HookFormTextField.collapsed(
              name: 'url',
              placeholder: 'https://...',
              style: TextStyle(fontSize: 16),
              autofocus: true,
              submitOnEnter: false,
              keyboardType: TextInputType.url,
            ),
          );
        },
      ),
    );
  }
}
