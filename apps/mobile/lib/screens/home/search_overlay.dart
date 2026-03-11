import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/extensions/iterable.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/debounce.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/home/__generated__/search_query.data.gql.dart';
import 'package:typie/screens/home/__generated__/search_query.req.gql.dart';
import 'package:typie/services/kv.dart';
import 'package:typie/services/site.dart';
import 'package:typie/widgets/horizontal_divider.dart';
import 'package:typie/widgets/tappable.dart';

class SearchOverlay extends HookWidget {
  const SearchOverlay({super.key, required this.active, required this.onExit});

  final bool active;
  final VoidCallback onExit;

  @override
  Widget build(BuildContext context) {
    final site = useService<Site>();
    final siteId = useValueListenable(site);
    final client = useService<GraphQLClient>();
    final kv = useService<KV>();

    final searchController = useTextEditingController();
    final searchFocusNode = useFocusNode();
    final searchData = useState<GSearchScreen_QueryData?>(null);
    final recentSearches = useState<List<String>>([]);
    final debounce = useDebounce<void>(const Duration(milliseconds: 300));
    final searchVersion = useRef(0);
    final searchText = useValueListenable(searchController);

    useEffect(() {
      if (active) {
        searchFocusNode.requestFocus();
      }
      return null;
    }, [active]);

    useEffect(() {
      var disposed = false;
      unawaited(
        kv.openBox('search').then((box) {
          if (!disposed) {
            recentSearches.value = (box.get('recent_searches') as List?)?.cast<String>() ?? [];
          }
        }),
      );
      return () => disposed = true;
    }, []);

    void doSearch(String query) {
      searchVersion.value++;
      debounce.cancel();
      if (query.isEmpty) {
        return;
      }
      final version = searchVersion.value;
      debounce.call(() async {
        try {
          final result = await client.request(
            GSearchScreen_QueryReq(
              (b) => b
                ..vars.siteId = siteId
                ..vars.query = query,
            ),
          );
          if (searchVersion.value == version) {
            searchData.value = result;
          }
        } catch (_) {}
      });
    }

    Future<void> saveRecentSearch(String query) async {
      if (query.trim().isEmpty) {
        return;
      }
      final box = await kv.openBox('search');
      final searches = ((box.get('recent_searches') as List?)?.cast<String>() ?? [])
        ..remove(query)
        ..insert(0, query);
      if (searches.length > 10) {
        searches.removeLast();
      }
      await box.put('recent_searches', searches);
      recentSearches.value = List.of(searches);
    }

    void exit() {
      searchController.clear();
      searchData.value = null;
      searchVersion.value++;
      searchFocusNode.unfocus();
      onExit();
    }

    return Column(
      children: [
        _SearchHeader(controller: searchController, focusNode: searchFocusNode, onChanged: doSearch, onCancel: exit),
        Expanded(
          child: _SearchResults(
            showRecentSearches: searchText.text.isEmpty,
            searchData: searchData.value,
            recentSearches: recentSearches.value,
            onSelectResult: (slug) {
              unawaited(saveRecentSearch(searchController.text));
              unawaited(context.router.push(NativeEditorRoute(slug: slug)));
            },
            onSelectRecent: (query) {
              searchController
                ..text = query
                ..selection = TextSelection.collapsed(offset: query.length);
              doSearch(query);
            },
          ),
        ),
      ],
    );
  }
}

// -- Search Header ------------------------------------------------------------

class _SearchHeader extends StatelessWidget {
  const _SearchHeader({
    required this.controller,
    required this.focusNode,
    required this.onChanged,
    required this.onCancel,
  });

  final TextEditingController controller;
  final FocusNode focusNode;
  final void Function(String) onChanged;
  final VoidCallback onCancel;

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
              Expanded(
                child: Container(
                  height: 36,
                  padding: const Pad(horizontal: 14),
                  decoration: BoxDecoration(
                    color: context.colors.surfaceDefault,
                    borderRadius: BorderRadius.circular(10),
                  ),
                  child: Row(
                    children: [
                      Icon(LucideLightIcons.search, size: 16, color: context.colors.accentBrand),
                      const Gap(10),
                      Expanded(
                        child: TextField(
                          controller: controller,
                          focusNode: focusNode,
                          onChanged: onChanged,
                          style: const TextStyle(fontSize: 15),
                          decoration: const InputDecoration(
                            hintText: '문서 검색...',
                            border: InputBorder.none,
                            isDense: true,
                            contentPadding: EdgeInsets.zero,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
              const Gap(14),
              Tappable(
                onTap: onCancel,
                child: Text(
                  '취소',
                  style: TextStyle(fontSize: 15, fontWeight: FontWeight.w500, color: context.colors.accentBrand),
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

// -- Search Results -----------------------------------------------------------

List<TextSpan> _parseEmHighlight(String text, TextStyle baseStyle, TextStyle highlightStyle) {
  final spans = <TextSpan>[];
  final regex = RegExp('<em>(.*?)</em>');
  var lastEnd = 0;

  for (final match in regex.allMatches(text)) {
    if (match.start > lastEnd) {
      spans.add(TextSpan(text: text.substring(lastEnd, match.start), style: baseStyle));
    }
    spans.add(TextSpan(text: match.group(1), style: highlightStyle));
    lastEnd = match.end;
  }

  if (lastEnd < text.length) {
    spans.add(TextSpan(text: text.substring(lastEnd), style: baseStyle));
  }

  return spans;
}

typedef _SearchHit = GSearchScreen_QueryData_search_hits__asSearchHitDocument;

class _SearchResults extends StatelessWidget {
  const _SearchResults({
    required this.showRecentSearches,
    required this.searchData,
    required this.recentSearches,
    required this.onSelectResult,
    required this.onSelectRecent,
  });

  final bool showRecentSearches;
  final GSearchScreen_QueryData? searchData;
  final List<String> recentSearches;
  final void Function(String slug) onSelectResult;
  final void Function(String query) onSelectRecent;

  @override
  Widget build(BuildContext context) {
    if (showRecentSearches) {
      return _RecentSearchesList(searches: recentSearches, onTap: onSelectRecent);
    }

    final hits = searchData?.search.hits.whereType<_SearchHit>().toList() ?? [];

    if (hits.isEmpty && searchData != null) {
      return Center(
        child: Text('검색 결과가 없습니다', style: TextStyle(fontSize: 15, color: context.colors.textFaint)),
      );
    }

    return SingleChildScrollView(
      padding: const Pad(horizontal: 20, top: 8),
      keyboardDismissBehavior: ScrollViewKeyboardDismissBehavior.onDrag,
      child: Container(
        decoration: BoxDecoration(color: context.colors.surfaceDefault, borderRadius: BorderRadius.circular(12)),
        clipBehavior: Clip.antiAlias,
        child: Column(
          children: hits
              .map(
                (hit) => Tappable(
                  onTap: () => onSelectResult(hit.document.entity.slug),
                  child: Padding(
                    padding: const Pad(horizontal: 16, vertical: 14),
                    child: Tappable.scale(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
                            children: [
                              Icon(
                                hit.document.type == GDocumentType.TEMPLATE
                                    ? LucideLightIcons.shapes
                                    : LucideLightIcons.file,
                                size: 16,
                                color: context.colors.textFaint,
                              ),
                              const Gap(12),
                              Expanded(
                                child: Text.rich(
                                  TextSpan(
                                    children: _parseEmHighlight(
                                      hit.title ?? '',
                                      const TextStyle(fontSize: 16, fontWeight: FontWeight.w600),
                                      TextStyle(
                                        fontSize: 16,
                                        fontWeight: FontWeight.w600,
                                        color: context.colors.textBrand,
                                      ),
                                    ),
                                  ),
                                  overflow: TextOverflow.ellipsis,
                                  maxLines: 1,
                                ),
                              ),
                            ],
                          ),
                          if (hit.text != null && hit.text!.isNotEmpty)
                            Padding(
                              padding: const Pad(left: 28),
                              child: Text.rich(
                                TextSpan(
                                  children: _parseEmHighlight(
                                    hit.text!,
                                    TextStyle(fontSize: 14, color: context.colors.textFaint),
                                    TextStyle(
                                      fontSize: 14,
                                      fontWeight: FontWeight.w600,
                                      color: context.colors.textBrand,
                                    ),
                                  ),
                                ),
                                overflow: TextOverflow.ellipsis,
                                maxLines: 1,
                              ),
                            ),
                        ],
                      ),
                    ),
                  ),
                ),
              )
              .intersperseWith(HorizontalDivider(color: context.colors.borderSubtle))
              .toList(),
        ),
      ),
    );
  }
}

// -- Recent Searches ----------------------------------------------------------

class _RecentSearchesList extends StatelessWidget {
  const _RecentSearchesList({required this.searches, required this.onTap});

  final List<String> searches;
  final void Function(String query) onTap;

  @override
  Widget build(BuildContext context) {
    if (searches.isEmpty) {
      return Center(
        child: Text('최근 검색어가 없습니다', style: TextStyle(fontSize: 15, color: context.colors.textFaint)),
      );
    }

    return SingleChildScrollView(
      keyboardDismissBehavior: ScrollViewKeyboardDismissBehavior.onDrag,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const Pad(horizontal: 20, top: 16, bottom: 12),
            child: Text(
              '최근 검색',
              style: TextStyle(
                fontSize: 13,
                fontWeight: FontWeight.w700,
                letterSpacing: 0.3,
                color: context.colors.textFaint,
              ),
            ),
          ),
          Padding(
            padding: const Pad(horizontal: 20),
            child: Container(
              decoration: BoxDecoration(color: context.colors.surfaceDefault, borderRadius: BorderRadius.circular(12)),
              clipBehavior: Clip.antiAlias,
              child: Column(
                children: searches
                    .map(
                      (search) => Tappable(
                        onTap: () => onTap(search),
                        child: Padding(
                          padding: const Pad(horizontal: 16, vertical: 14),
                          child: Row(
                            children: [
                              Icon(LucideLightIcons.clock_3, size: 16, color: context.colors.textFaint),
                              const Gap(12),
                              Expanded(
                                child: Text(
                                  search,
                                  style: TextStyle(fontSize: 15, color: context.colors.textSubtle),
                                  overflow: TextOverflow.ellipsis,
                                  maxLines: 1,
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),
                    )
                    .intersperseWith(HorizontalDivider(color: context.colors.borderSubtle))
                    .toList(),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
