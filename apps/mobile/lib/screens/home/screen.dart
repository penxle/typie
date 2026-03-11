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
import 'package:typie/graphql/hooks.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/home/__generated__/query.data.gql.dart';
import 'package:typie/screens/home/__generated__/query.req.gql.dart';
import 'package:typie/screens/home/search_overlay.dart';
import 'package:typie/screens/shell/screen.dart';
import 'package:typie/services/site.dart';
import 'package:typie/widgets/horizontal_divider.dart';
import 'package:typie/widgets/img.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';

const _animDuration = Duration(milliseconds: 350);
const _animCurve = Curves.easeOutCubic;

@RoutePage()
class HomeScreen extends HookWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final site = useService<Site>();
    final siteId = useValueListenable(site);

    final data = useQuery(GHomeScreen_QueryReq((b) => b.vars.siteId = siteId));

    final scrollController = useScrollController();
    final showHeadingTitle = useState(false);
    final isSearching = useState(false);

    useEffect(() {
      void onScroll() {
        showHeadingTitle.value = scrollController.offset > 44;
      }

      scrollController.addListener(onScroll);
      return () => scrollController.removeListener(onScroll);
    }, [scrollController]);

    void enterSearch() {
      isSearching.value = true;
      ShellNav.of(context).hide();
    }

    void exitSearch() {
      isSearching.value = false;
      ShellNav.of(context).show();
    }

    final searching = isSearching.value;

    return PopScope(
      canPop: !searching,
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop) {
          exitSearch();
        }
      },
      child: Screen(
        loading: data == null,
        resizeToAvoidBottomInset: searching,
        child: Stack(
          children: [
            // -- Home content --
            AnimatedOpacity(
              opacity: searching ? 0 : 1,
              duration: const Duration(milliseconds: 200),
              child: IgnorePointer(
                ignoring: searching,
                child: SingleChildScrollView(
                  controller: scrollController,
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      const Gap(72),
                      const Padding(
                        padding: Pad(horizontal: 20, top: 8, bottom: 4),
                        child: Text('홈', style: TextStyle(fontSize: 24, fontWeight: FontWeight.w800)),
                      ),
                      _SearchBarPlaceholder(onTap: enterSearch),
                      _RecentFolders(data: data),
                      _RecentDocuments(data: data),
                      const Gap(140),
                    ],
                  ),
                ),
              ),
            ),

            // -- Search overlay --
            AnimatedOpacity(
              opacity: searching ? 1 : 0,
              duration: Duration(milliseconds: searching ? 250 : 150),
              child: IgnorePointer(
                ignoring: !searching,
                child: SearchOverlay(active: searching, onExit: exitSearch),
              ),
            ),

            // -- Heading (slides out when searching) --
            AnimatedSlide(
              offset: Offset(0, searching ? -1.0 : 0),
              duration: _animDuration,
              curve: _animCurve,
              child: AnimatedOpacity(
                opacity: searching ? 0 : 1,
                duration: const Duration(milliseconds: 200),
                child: _Heading(data: data, showTitle: showHeadingTitle.value),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

// -- Search Bar Placeholder ---------------------------------------------------

class _SearchBarPlaceholder extends StatelessWidget {
  const _SearchBarPlaceholder({required this.onTap});

  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const Pad(horizontal: 20, top: 12, bottom: 4),
      child: Tappable(
        onTap: onTap,
        child: Container(
          height: 36,
          padding: const Pad(horizontal: 14),
          decoration: BoxDecoration(color: context.colors.surfaceDefault, borderRadius: BorderRadius.circular(10)),
          child: Row(
            children: [
              Icon(LucideLightIcons.search, size: 16, color: context.colors.textDisabled),
              const Gap(10),
              Text('문서 검색...', style: TextStyle(fontSize: 15, color: context.colors.textDisabled)),
            ],
          ),
        ),
      ),
    );
  }
}

// -- Heading ------------------------------------------------------------------

class _Heading extends StatelessWidget {
  const _Heading({required this.data, required this.showTitle});

  final GHomeScreen_QueryData? data;
  final bool showTitle;

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
          child: Stack(
            alignment: Alignment.center,
            children: [
              Align(
                alignment: Alignment.centerLeft,
                child: Tappable(
                  onTap: () {
                    unawaited(context.router.push(const SiteRoute()));
                  },
                  child: Tappable.scale(
                    scale: 0.95,
                    child: Transform.rotate(
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
                  ),
                ),
              ),
              AnimatedSlide(
                offset: Offset(0, showTitle ? 0.0 : 0.4),
                duration: const Duration(milliseconds: 200),
                curve: Curves.easeOut,
                child: AnimatedOpacity(
                  opacity: showTitle ? 1.0 : 0.0,
                  duration: const Duration(milliseconds: 150),
                  child: const Text('홈', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w700)),
                ),
              ),
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

// -- Recent Folders -----------------------------------------------------------

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
            style: TextStyle(
              fontSize: 13,
              fontWeight: FontWeight.w700,
              letterSpacing: 0.3,
              color: context.colors.textFaint,
            ),
          ),
        ),
        SizedBox(
          height: 110,
          child: folders.isEmpty
              ? Container(
                  margin: const Pad(horizontal: 20),
                  alignment: Alignment.center,
                  decoration: BoxDecoration(
                    color: context.colors.surfaceDefault,
                    borderRadius: BorderRadius.circular(12),
                  ),
                  child: Text('최근 사용한 폴더가 여기 나타나요', style: TextStyle(fontSize: 15, color: context.colors.textFaint)),
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
                            // ignore: avoid_unnecessary_containers -- false positive
                            child: Container(
                              width: 140,
                              padding: const Pad(all: 16),
                              decoration: BoxDecoration(
                                color: context.colors.surfaceDefault,
                                borderRadius: BorderRadius.circular(12),
                              ),
                              child: Tappable.scale(
                                child: Column(
                                  crossAxisAlignment: CrossAxisAlignment.start,
                                  children: [
                                    Icon(LucideLightIcons.folder, size: 18, color: context.colors.accentBrand),
                                    const Gap(6),
                                    Text(
                                      folder.name,
                                      style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w600),
                                      overflow: TextOverflow.ellipsis,
                                      maxLines: 1,
                                    ),
                                    const Gap(2),
                                    Text(
                                      '문서 ${folder.documentCount}개',
                                      style: TextStyle(
                                        fontSize: 13,
                                        fontWeight: FontWeight.w500,
                                        color: context.colors.textFaint,
                                      ),
                                    ),
                                  ],
                                ),
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

// -- Recent Documents ---------------------------------------------------------

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

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const Pad(horizontal: 20, top: 24, bottom: 12),
          child: Text(
            '최근 문서',
            style: TextStyle(
              fontSize: 13,
              fontWeight: FontWeight.w700,
              letterSpacing: 0.3,
              color: context.colors.textFaint,
            ),
          ),
        ),
        if (docs.isNotEmpty)
          Padding(
            padding: const Pad(horizontal: 20),
            child: Container(
              decoration: BoxDecoration(color: context.colors.surfaceDefault, borderRadius: BorderRadius.circular(12)),
              clipBehavior: Clip.antiAlias,
              child: Column(
                children: docs
                    .map((doc) => _DocumentRow(doc: doc))
                    .intersperseWith(HorizontalDivider(color: context.colors.borderSubtle))
                    .toList(),
              ),
            ),
          ),
      ],
    );
  }
}

class _DocumentRow extends StatelessWidget {
  const _DocumentRow({required this.doc});

  final _RecentDocument doc;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      onTap: () {
        unawaited(context.router.push(NativeEditorRoute(slug: doc.entity.slug)));
      },
      child: Padding(
        padding: const Pad(horizontal: 16, vertical: 14),
        child: Tappable.scale(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Icon(
                    doc.type == GDocumentType.TEMPLATE ? LucideLightIcons.shapes : LucideLightIcons.file,
                    size: 16,
                    color: context.colors.textFaint,
                  ),
                  const Gap(12),
                  Expanded(
                    child: Text(
                      doc.title,
                      style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w600),
                      overflow: TextOverflow.ellipsis,
                      maxLines: 1,
                    ),
                  ),
                  const Gap(8),
                  Text(
                    doc.updatedAt.ago,
                    style: TextStyle(fontSize: 13, fontWeight: FontWeight.w500, color: context.colors.textDisabled),
                  ),
                ],
              ),
              if (doc.excerpt.isNotEmpty)
                Padding(
                  padding: const Pad(left: 28),
                  child: Text(
                    doc.excerpt,
                    style: TextStyle(fontSize: 14, color: context.colors.textFaint),
                    overflow: TextOverflow.ellipsis,
                    maxLines: 1,
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }
}
