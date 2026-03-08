import 'dart:math';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/loader.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/entity/__generated__/create_site_mutation.req.gql.dart';
import 'package:typie/screens/entity/__generated__/space_selector_query.req.gql.dart';
import 'package:typie/screens/native_editor/limit.dart';
import 'package:typie/services/site.dart';
import 'package:typie/widgets/tappable.dart';

class SpaceSelectorBottomSheet extends HookWidget {
  const SpaceSelectorBottomSheet({super.key, this.onSiteChanged});

  final VoidCallback? onSiteChanged;

  @override
  Widget build(BuildContext context) {
    final site = useService<Site>();
    final currentSiteId = useValueListenable(site);

    return GraphQLOperation(
      operation: GSpaceSelector_QueryReq(),
      builder: (context, client, data) {
        final me = data.me!;
        final sites = me.sites.toList();
        final hasSubscription = me.subscription != null;

        return AppBottomSheet(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const Padding(
                padding: Pad(horizontal: 24),
                child: Text('스페이스', style: TextStyle(fontSize: 17, fontWeight: FontWeight.w600)),
              ),
              const SizedBox(height: 16),
              ...sites.map((s) {
                final isSelected = s.id == currentSiteId;
                const logoSize = 24.0;
                final imageSize = pow(
                  2,
                  (log(logoSize * MediaQuery.devicePixelRatioOf(context)) / log(2)).ceil(),
                ).toInt();

                return Tappable(
                  padding: const Pad(horizontal: 24, vertical: 10),
                  onTap: () {
                    if (!isSelected) {
                      site.setSiteId(s.id);
                      onSiteChanged?.call();
                    }
                    context.router.pop();
                  },
                  child: Row(
                    spacing: 12,
                    children: [
                      ClipRRect(
                        borderRadius: BorderRadius.circular(4),
                        child: CachedNetworkImage(
                          imageUrl: '${s.logo.url}?s=$imageSize&q=75',
                          width: logoSize,
                          height: logoSize,
                          fit: BoxFit.cover,
                        ),
                      ),
                      Expanded(
                        child: Text(
                          s.name,
                          style: TextStyle(fontSize: 16, fontWeight: isSelected ? FontWeight.w600 : FontWeight.w400),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                      if (isSelected) Icon(LucideLightIcons.check, size: 18, color: context.colors.textDefault),
                    ],
                  ),
                );
              }),
              Padding(
                padding: const Pad(vertical: 8),
                child: Divider(height: 1, color: context.colors.borderDefault),
              ),
              Tappable(
                padding: const Pad(horizontal: 24, vertical: 10),
                onTap: () async {
                  context.router.pop();

                  if (!hasSubscription) {
                    if (context.mounted) {
                      await context.showBottomSheet(
                        child: const LimitBottomSheet(type: LimitBottomSheetType.multiSite),
                      );
                    }
                    return;
                  }

                  if (context.mounted) {
                    await context.showBottomSheet(
                      child: _CreateSiteBottomSheet(client: client, site: site, onSiteChanged: onSiteChanged),
                    );
                  }
                },
                child: Row(
                  spacing: 12,
                  children: [
                    Icon(LucideLightIcons.plus, size: 24, color: context.colors.textSubtle),
                    Text('새 스페이스 생성', style: TextStyle(fontSize: 16, color: context.colors.textSubtle)),
                  ],
                ),
              ),
            ],
          ),
        );
      },
    );
  }
}

class _CreateSiteBottomSheet extends HookWidget {
  const _CreateSiteBottomSheet({required this.client, required this.site, this.onSiteChanged});

  final GraphQLClient client;
  final Site site;
  final VoidCallback? onSiteChanged;

  @override
  Widget build(BuildContext context) {
    final controller = useTextEditingController();

    return ConfirmBottomSheet(
      title: '새 스페이스 생성',
      message: '스페이스는 독립된 글쓰기 공간이에요.\n주제나 목적에 따라 글을 나누어 관리해보세요.',
      confirmText: '생성',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 6,
        children: [
          Text(
            '스페이스 이름',
            style: TextStyle(fontSize: 13, fontWeight: FontWeight.w500, color: context.colors.textDefault),
          ),
          TextField(
            controller: controller,
            autofocus: true,
            decoration: InputDecoration(
              hintText: '새 스페이스',
              hintStyle: TextStyle(color: context.colors.textDisabled),
              border: OutlineInputBorder(borderRadius: BorderRadius.circular(8)),
              contentPadding: const Pad(horizontal: 12, vertical: 10),
            ),
            style: const TextStyle(fontSize: 16),
          ),
        ],
      ),
      onConfirm: () async {
        final name = controller.text.trim();

        try {
          await context.runWithLoader(() async {
            final result = await client.request(
              GSpaceSelector_CreateSite_MutationReq((b) => b..vars.input.name = name.isEmpty ? '새 스페이스' : name),
            );

            site.setSiteId(result.createSite.id);
            onSiteChanged?.call();
          });

          if (context.mounted) {
            context.toast(ToastType.success, '새 스페이스가 생성되었어요.');
          }
        } catch (_) {
          if (context.mounted) {
            context.toast(ToastType.error, '오류가 발생했어요. 잠시 후 다시 시도해주세요.');
          }
        }
      },
    );
  }
}
