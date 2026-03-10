import 'dart:async';
import 'dart:math' as math;

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:jiffy/jiffy.dart';
import 'package:skeletonizer/skeletonizer.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/extensions/iterable.dart';
import 'package:typie/extensions/jiffy.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/hook.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/home/__generated__/query.data.gql.dart';
import 'package:typie/screens/home/__generated__/query.req.gql.dart';
import 'package:typie/services/site.dart';
import 'package:typie/widgets/img.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';

@RoutePage()
class HomeScreen extends HookWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final site = useService<Site>();
    final siteId = useValueListenable(site);

    final data = useQuery(GHomeScreen_QueryReq((b) => b.vars.siteId = siteId));

    return Screen(
      loading: data == null,
      child: Stack(
        children: [
          SingleChildScrollView(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                const Gap(48),
                _RecentFolders(data: data),
                _RecentDocuments(data: data),
                const Gap(40),
              ],
            ),
          ),
          _Heading(data: data),
        ],
      ),
    );
  }
}

class _Heading extends StatelessWidget {
  const _Heading({required this.data});

  final GHomeScreen_QueryData? data;

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Container(
          height: 48,
          padding: const Pad(horizontal: 20),
          decoration: BoxDecoration(color: context.colors.surfaceSubtle),
          child: Row(
            children: [
              Transform.rotate(
                angle: -10 * (math.pi / 180),
                child: Container(
                  decoration: BoxDecoration(
                    borderRadius: BorderRadius.circular(6),
                    boxShadow: [
                      BoxShadow(
                        color: context.colors.shadowDefault.withValues(alpha: 0.08),
                        blurRadius: 4,
                        offset: const Offset(0, 1),
                      ),
                    ],
                  ),
                  child: ClipRRect(
                    borderRadius: BorderRadius.circular(6),
                    child: Img(image: data?.site.logo, size: 36),
                  ),
                ),
              ),
              const Spacer(),
            ],
          ),
        ),
        Container(
          height: 24,
          decoration: BoxDecoration(
            gradient: LinearGradient(
              begin: Alignment.topCenter,
              end: Alignment.bottomCenter,
              colors: [context.colors.surfaceSubtle, context.colors.surfaceSubtle.withValues(alpha: 0)],
            ),
          ),
        ),
      ],
    );
  }
}

typedef _RecentFolder = GHomeScreen_QueryData_me_recentlyViewedEntities_node__asFolder;

class _RecentFolders extends StatelessWidget {
  const _RecentFolders({required this.data});

  final GHomeScreen_QueryData? data;

  @override
  Widget build(BuildContext context) {
    final folders =
        data?.me?.recentlyViewedEntities.ofType<_RecentFolder>((v) => v.node).toList() ??
        List.filled(
          4,
          _RecentFolder(
            (b) => b
              ..id = ''
              ..name = BoneMock.title
              ..documentCount = 0
              ..entity.id = ''
              ..entity.slug = '',
          ),
        );

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const Pad(horizontal: 20, top: 20, bottom: 12),
          child: Text(
            '최근 폴더',
            style: TextStyle(fontSize: 12, fontWeight: FontWeight.w600, color: context.colors.textFaint),
          ),
        ),
        SizedBox(
          height: 100,
          child: folders.isEmpty
              ? Container(
                  margin: const Pad(horizontal: 20),
                  alignment: Alignment.center,
                  decoration: BoxDecoration(
                    color: context.colors.surfaceDefault,
                    borderRadius: BorderRadius.circular(12),
                  ),
                  child: Text('최근 사용한 폴더가 여기 나타나요', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
                )
              : SingleChildScrollView(
                  scrollDirection: Axis.horizontal,
                  physics: const AlwaysScrollableScrollPhysics(),
                  padding: const Pad(horizontal: 20),
                  child: Row(
                    spacing: 16,
                    children: folders
                        .map(
                          (folder) => Tappable(
                            onTap: () async {
                              await context.router.push(EntityRoute(entityId: folder.id));
                            },
                            child: Container(
                              width: 140,
                              padding: const Pad(all: 16),
                              decoration: BoxDecoration(
                                color: context.colors.surfaceDefault,
                                borderRadius: BorderRadius.circular(12),
                              ),
                              child: Column(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                children: [
                                  Icon(LucideLightIcons.folder, size: 18, color: context.colors.accentBrand),
                                  const Gap(6),
                                  Text(
                                    folder.name,
                                    style: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600),
                                    overflow: TextOverflow.ellipsis,
                                    maxLines: 1,
                                  ),
                                  const Gap(2),
                                  Text(
                                    '문서 ${folder.documentCount}개',
                                    style: TextStyle(fontSize: 11, color: context.colors.textFaint),
                                  ),
                                ],
                              ),
                            ),
                          ),
                        )
                        .toList(),
                  ),
                ),
        ),
      ],
    );
  }
}

// -- Recent Documents (hybrid: cards + list) ----------------------------------

typedef _RecentDocument = GHomeScreen_QueryData_me_recentlyViewedEntities_node__asDocument;

class _RecentDocuments extends StatelessWidget {
  const _RecentDocuments({required this.data});

  final GHomeScreen_QueryData? data;

  @override
  Widget build(BuildContext context) {
    final docs =
        data?.me?.recentlyViewedEntities.ofType<_RecentDocument>((v) => v.node).toList() ??
        List.filled(
          10,
          _RecentDocument(
            (b) => b
              ..id = ''
              ..title = BoneMock.title
              ..excerpt = BoneMock.subtitle
              ..updatedAt = Jiffy.now()
              ..type = GDocumentType.NORMAL
              ..entity.id = ''
              ..entity.slug = '',
          ),
        );

    final topDocs = docs.take(2).toList();
    final restDocs = docs.skip(2).toList();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const Pad(horizontal: 20, top: 24, bottom: 12),
          child: Text(
            '최근 문서',
            style: TextStyle(fontSize: 12, fontWeight: FontWeight.w600, color: context.colors.textFaint),
          ),
        ),

        // Top cards
        Padding(
          padding: const Pad(horizontal: 20),
          child: IntrinsicHeight(
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                for (var i = 0; i < topDocs.length; i++) ...[
                  if (i > 0) const Gap(12),
                  Expanded(child: _DocumentCard(doc: topDocs[i])),
                ],
                if (topDocs.length == 1) ...[const Gap(12), const Expanded(child: SizedBox.shrink())],
              ],
            ),
          ),
        ),

        // Compact list
        if (restDocs.isNotEmpty)
          Padding(
            padding: const Pad(horizontal: 20, top: 12),
            child: Container(
              decoration: BoxDecoration(color: context.colors.surfaceDefault, borderRadius: BorderRadius.circular(12)),
              clipBehavior: Clip.antiAlias,
              child: Column(
                children: [
                  for (var i = 0; i < restDocs.length; i++)
                    _CompactDocumentRow(doc: restDocs[i], isLast: i == restDocs.length - 1),
                ],
              ),
            ),
          ),
      ],
    );
  }
}

class _DocumentCard extends StatelessWidget {
  const _DocumentCard({required this.doc});

  final _RecentDocument doc;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: () {
        unawaited(context.router.push(NativeEditorRoute(slug: doc.entity.slug)));
      },
      child: Container(
        padding: const Pad(all: 16),
        decoration: BoxDecoration(color: context.colors.surfaceDefault, borderRadius: BorderRadius.circular(12)),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              doc.title,
              style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w600),
              overflow: TextOverflow.ellipsis,
              maxLines: 1,
            ),
            const Gap(6),
            Text(
              doc.excerpt.isEmpty ? '(내용 없음)' : doc.excerpt,
              style: TextStyle(fontSize: 12, color: context.colors.textFaint, height: 1.4),
              overflow: TextOverflow.ellipsis,
              maxLines: 2,
            ),
            const Gap(10),
            Text(doc.updatedAt.ago, style: TextStyle(fontSize: 11, color: context.colors.textDisabled)),
          ],
        ),
      ),
    );
  }
}

class _CompactDocumentRow extends StatelessWidget {
  const _CompactDocumentRow({required this.doc, required this.isLast});

  final _RecentDocument doc;
  final bool isLast;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: () {
        unawaited(context.router.push(NativeEditorRoute(slug: doc.entity.slug)));
      },
      child: Container(
        padding: const Pad(horizontal: 16, vertical: 14),
        decoration: BoxDecoration(
          border: isLast ? null : Border(bottom: BorderSide(color: context.colors.borderSubtle, width: 0.5)),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(
                  doc.type == GDocumentType.TEMPLATE ? LucideLightIcons.shapes : LucideLightIcons.file,
                  size: 14,
                  color: context.colors.accentBrand,
                ),
                const Gap(12),
                Expanded(
                  child: Text(
                    doc.title,
                    style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w500),
                    overflow: TextOverflow.ellipsis,
                    maxLines: 1,
                  ),
                ),
                const Gap(8),
                Text(doc.updatedAt.ago, style: TextStyle(fontSize: 11, color: context.colors.textDisabled)),
              ],
            ),
            if (doc.excerpt.isNotEmpty)
              Padding(
                padding: const Pad(left: 26),
                child: Text(
                  doc.excerpt,
                  style: TextStyle(fontSize: 12, color: context.colors.textFaint),
                  overflow: TextOverflow.ellipsis,
                  maxLines: 1,
                ),
              ),
          ],
        ),
      ),
    );
  }
}
