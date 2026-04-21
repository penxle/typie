import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/native_editor/external/models.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';

class EmbedWidget extends HookWidget {
  const EmbedWidget({required this.element, super.key});

  final ExternalElement element;

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final uploadManager = scope.uploadManager;

    useListenable(uploadManager);

    final embedData = element.data as EmbedElementData;

    final asset = embedData.id != null ? uploadManager.embedAssets[embedData.id] : null;
    final isInflight = uploadManager.inflightEmbeds[element.nodeId] ?? false;

    final hasEmbed = asset != null || isInflight;

    if (!hasEmbed) {
      return _buildPlaceholder(context);
    }

    if (isInflight && asset == null) {
      return _buildLoading(context);
    }

    if (asset != null) {
      return _buildCard(context, asset);
    }

    return _buildPlaceholder(context);
  }

  Widget _buildPlaceholder(BuildContext context) {
    return Container(
      height: 48,
      decoration: BoxDecoration(color: context.colors.surfaceMuted, borderRadius: BorderRadius.circular(4)),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
        child: Row(
          children: [
            Icon(LucideLightIcons.file_up, size: 20, color: context.colors.textDisabled),
            const SizedBox(width: 12),
            Expanded(
              child: Text(
                '링크 임베드(Youtube, Google Drive, 일반 링크 등)',
                style: TextStyle(fontSize: 14, color: context.colors.textDisabled),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildLoading(BuildContext context) {
    return Container(
      height: 48,
      decoration: BoxDecoration(color: context.colors.surfaceMuted, borderRadius: BorderRadius.circular(4)),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
        child: Row(
          children: [
            SizedBox(
              width: 20,
              height: 20,
              child: CircularProgressIndicator(strokeWidth: 2, color: context.colors.textDisabled),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Text(
                '링크 임베드 중...',
                style: TextStyle(fontSize: 14, color: context.colors.textDisabled),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildCard(BuildContext context, EmbedAsset asset) {
    final host = Uri.tryParse(asset.url)?.host ?? asset.url;

    return Container(
      constraints: const BoxConstraints(maxWidth: 600),
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(6),
        border: Border.all(color: context.colors.borderSubtle),
      ),
      child: Row(
        children: [
          Expanded(
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 15),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(
                    asset.title ?? '(제목 없음)',
                    style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textDefault),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                  if (asset.description != null && asset.description!.isNotEmpty) ...[
                    const SizedBox(height: 3),
                    Text(
                      asset.description!,
                      style: TextStyle(fontSize: 12, fontWeight: FontWeight.w500, color: context.colors.textFaint),
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ],
                  const SizedBox(height: 8),
                  Text(
                    host,
                    style: TextStyle(fontSize: 12, fontWeight: FontWeight.w500, color: context.colors.textDefault),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ],
              ),
            ),
          ),
          if (asset.thumbnailUrl != null)
            ClipRRect(
              borderRadius: const BorderRadius.only(topRight: Radius.circular(5), bottomRight: Radius.circular(5)),
              child: SizedBox(
                width: 118,
                height: 118,
                child: CachedNetworkImage(
                  imageUrl: asset.thumbnailUrl!,
                  fit: BoxFit.cover,
                  fadeInDuration: const Duration(milliseconds: 150),
                  errorWidget: (context, url, error) => const SizedBox.shrink(),
                ),
              ),
            ),
        ],
      ),
    );
  }
}
