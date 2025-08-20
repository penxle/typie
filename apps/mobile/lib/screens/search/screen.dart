import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/extensions/jiffy.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/search/__generated__/recently_viewed_query.data.gql.dart';
import 'package:typie/screens/search/__generated__/recently_viewed_query.req.gql.dart';
import 'package:typie/screens/search/__generated__/search_query.data.gql.dart';
import 'package:typie/screens/search/__generated__/search_query.req.gql.dart';
import 'package:typie/services/preference.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';

@RoutePage()
class SearchScreen extends HookWidget {
  const SearchScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final pref = useService<Pref>();
    final controller = useTextEditingController();

    final value = useValueListenable(controller);

    final recentlyViewedRefreshNotifier = useMemoized(RefreshNotifier.new);

    useEffect(() {
      return recentlyViewedRefreshNotifier.dispose;
    }, []);

    // NOTE: 검색어 삭제 시 최근 본 항목 새로고침
    final previousTextWasEmpty = useRef(true);
    useEffect(() {
      if (value.text.isEmpty && !previousTextWasEmpty.value) {
        recentlyViewedRefreshNotifier.refresh();
      }
      previousTextWasEmpty.value = value.text.isEmpty;
      return null;
    }, [value.text]);

    // NOTE: 서치 탭으로 오면 최근 본 항목 새로고침
    useEffect(() {
      final tabsRouter = AutoTabsRouter.of(context);

      void onTabChange() {
        if (tabsRouter.activeIndex == 1 && value.text.isEmpty) {
          recentlyViewedRefreshNotifier.refresh();
        }
      }

      tabsRouter.addListener(onTabChange);
      return () => tabsRouter.removeListener(onTabChange);
    }, [value.text]);

    return Screen(
      heading: Heading(
        titleIcon: LucideLightIcons.search,
        titleWidget: Padding(
          padding: const Pad(left: 4),
          child: TextField(
            controller: controller,
            textInputAction: TextInputAction.search,
            decoration: InputDecoration.collapsed(
              hintText: '검색어를 입력하세요',
              hintStyle: TextStyle(fontSize: 16, fontWeight: FontWeight.w500, color: context.colors.textDisabled),
            ),
            style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w500),
          ),
        ),
      ),
      child: value.text.isEmpty
          ? GraphQLOperation(
              operation: GSearchScreen_RecentlyViewed_QueryReq(),
              refreshNotifier: recentlyViewedRefreshNotifier,
              builder: (context, client, data) {
                if (data.me?.recentlyViewedEntities.isEmpty ?? true) {
                  return Center(
                    child: Text('검색어를 입력해주세요', style: TextStyle(fontSize: 15, color: context.colors.textFaint)),
                  );
                }

                return ListView(
                  padding: const Pad(all: 20),
                  children: [
                    Text(
                      '최근 본 항목',
                      style: TextStyle(fontSize: 15, fontWeight: FontWeight.w500, color: context.colors.textFaint),
                    ),
                    const Gap(4),
                    ...data.me!.recentlyViewedEntities
                        .take(5)
                        .map(
                          (entity) => Padding(
                            padding: const Pad(vertical: 6),
                            child: _RecentlyViewedItem(entity: entity, refreshNotifier: recentlyViewedRefreshNotifier),
                          ),
                        ),
                  ],
                );
              },
            )
          : GraphQLOperation(
              operation: GSearchScreen_QueryReq(
                (b) => b
                  ..vars.siteId = pref.siteId
                  ..vars.query = value.text,
              ),
              builder: (context, client, data) {
                if (data.search.hits.isEmpty) {
                  return Center(
                    child: Text('검색 결과가 없어요', style: TextStyle(fontSize: 15, color: context.colors.textFaint)),
                  );
                }

                return ListView.separated(
                  padding: const Pad(all: 20),
                  itemCount: data.search.hits.length,
                  itemBuilder: (context, index) {
                    return data.search.hits[index].when(
                      searchHitPost: (post) => Tappable(
                        onTap: () async {
                          await context.router.push(EditorRoute(slug: post.post.entity.slug));
                        },
                        child: Container(
                          decoration: BoxDecoration(
                            border: Border.all(color: context.colors.borderStrong),
                            borderRadius: BorderRadius.circular(8),
                            color: context.colors.surfaceDefault,
                          ),
                          padding: const Pad(horizontal: 16, vertical: 12),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.stretch,
                            spacing: 4,
                            children: [
                              Row(
                                spacing: 8,
                                children: [
                                  if (post.post.type == GPostType.TEMPLATE)
                                    const Icon(LucideLightIcons.shapes, size: 18),
                                  Expanded(
                                    child: _HTMLText(
                                      post.title ?? '(제목 없음)',
                                      style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w500),
                                    ),
                                  ),
                                  Text(
                                    post.post.updatedAt.ago,
                                    style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
                                  ),
                                ],
                              ),
                              _HTMLText(
                                post.text ?? '(내용 없음)',
                                style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
                              ),
                            ],
                          ),
                        ),
                      ),
                      searchHitCanvas: (canvas) => Tappable(
                        onTap: () async {
                          await context.router.push(CanvasRoute(slug: canvas.canvas.entity.slug));
                        },
                        child: Container(
                          decoration: BoxDecoration(
                            border: Border.all(color: context.colors.borderStrong),
                            borderRadius: BorderRadius.circular(8),
                            color: context.colors.surfaceDefault,
                          ),
                          padding: const Pad(horizontal: 16, vertical: 12),
                          child: Row(
                            spacing: 8,
                            children: [
                              const Icon(LucideLightIcons.line_squiggle, size: 18),
                              Expanded(
                                child: _HTMLText(
                                  canvas.title ?? '(제목 없음)',
                                  style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w500),
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),
                      orElse: () => throw UnimplementedError(),
                    );
                  },
                  separatorBuilder: (context, index) {
                    return const Gap(12);
                  },
                );
              },
            ),
    );
  }
}

class _RecentlyViewedItem extends StatelessWidget {
  const _RecentlyViewedItem({required this.entity, this.refreshNotifier});

  final GSearchScreen_RecentlyViewed_QueryData_me_recentlyViewedEntities entity;
  final RefreshNotifier? refreshNotifier;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: () async {
        await entity.node.when(
          post: (_) => context.router.push(EditorRoute(slug: entity.slug)),
          canvas: (_) => context.router.push(CanvasRoute(slug: entity.slug)),
          orElse: Future<void>.value,
        );
        refreshNotifier?.refresh();
      },
      child: Container(
        decoration: BoxDecoration(
          border: Border.all(color: context.colors.borderStrong),
          borderRadius: const BorderRadius.all(Radius.circular(8)),
          color: context.colors.surfaceDefault,
        ),
        padding: const Pad(horizontal: 16, vertical: 12),
        child: entity.node.when(
          post: (post) => Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            spacing: 4,
            children: [
              Row(
                spacing: 8,
                children: [
                  if (post.type == GPostType.TEMPLATE) const Icon(LucideLightIcons.shapes, size: 18),
                  Expanded(
                    child: Text(
                      post.title,
                      style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w500),
                      overflow: TextOverflow.ellipsis,
                      maxLines: 1,
                    ),
                  ),
                  Text(post.updatedAt.ago, style: TextStyle(fontSize: 14, color: context.colors.textSubtle)),
                ],
              ),
              Text(
                post.excerpt.isEmpty ? '(내용 없음)' : post.excerpt,
                style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
                overflow: TextOverflow.ellipsis,
                maxLines: 1,
              ),
            ],
          ),
          canvas: (canvas) => Row(
            spacing: 8,
            children: [
              const Icon(LucideLightIcons.line_squiggle, size: 18),
              Expanded(
                child: Text(
                  canvas.title,
                  style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w500),
                  overflow: TextOverflow.ellipsis,
                  maxLines: 1,
                ),
              ),
            ],
          ),
          orElse: () => const SizedBox.shrink(),
        ),
      ),
    );
  }
}

class _HTMLText extends StatelessWidget {
  const _HTMLText(this.text, {this.style});

  final String text;
  final TextStyle? style;

  TextSpan _buildTextSpan(BuildContext context, String input) {
    final emRegExp = RegExp('<em>(.*?)</em>');
    final spans = <InlineSpan>[];
    var currentIndex = 0;

    for (final match in emRegExp.allMatches(input)) {
      if (match.start > currentIndex) {
        spans.add(
          TextSpan(
            text: input.substring(currentIndex, match.start),
            style: style ?? TextStyle(color: context.colors.textDefault),
          ),
        );
      }

      spans.add(
        TextSpan(
          text: match.group(1),
          style: (style ?? TextStyle(color: context.colors.textDefault)).copyWith(
            fontWeight: FontWeight.w700,
            color: context.colors.textDefault,
          ),
        ),
      );

      currentIndex = match.end;
    }

    if (currentIndex < input.length) {
      spans.add(
        TextSpan(
          text: input.substring(currentIndex),
          style: style ?? TextStyle(color: context.colors.textDefault),
        ),
      );
    }

    return TextSpan(children: spans);
  }

  @override
  Widget build(BuildContext context) {
    return Text.rich(_buildTextSpan(context, text), maxLines: 1, overflow: TextOverflow.ellipsis);
  }
}
