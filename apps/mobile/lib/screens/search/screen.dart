import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/search/__generated__/screen.data.gql.dart';
import 'package:typie/screens/search/__generated__/screen.req.gql.dart';
import 'package:typie/services/preference.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';

@RoutePage()
class SearchScreen extends HookWidget {
  const SearchScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final pref = useService<Pref>();
    final query = useState('');

    return Screen(
      heading: Heading(
        titleIcon: LucideLightIcons.search,
        titleWidget: Padding(
          padding: const Pad(left: 4),
          child: TextField(
            decoration: const InputDecoration.collapsed(
              hintText: '검색어를 입력하세요',
              hintStyle: TextStyle(fontSize: 16, fontWeight: FontWeight.w500, color: AppColors.gray_300),
            ),
            style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w500),
            onChanged: (value) {
              query.value = value;
            },
          ),
        ),
      ),
      child: GraphQLOperation(
        operation: GSearchScreen_QueryReq(
          (b) => b
            ..vars.siteId = pref.siteId
            ..vars.query = query.value,
        ),
        builder: (context, client, data) {
          if (query.value.isEmpty) {
            return const Center(
              child: Text('검색어를 입력해주세요', style: TextStyle(fontSize: 15, color: AppColors.gray_700)),
            );
          }

          if (data.search.hits.isEmpty) {
            return const Center(
              child: Text('검색 결과가 없어요', style: TextStyle(fontSize: 15, color: AppColors.gray_700)),
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
                      border: Border.all(color: AppColors.gray_950),
                      borderRadius: BorderRadius.circular(8),
                      color: AppColors.white,
                    ),
                    padding: const Pad(horizontal: 16, vertical: 12),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      spacing: 4,
                      children: [
                        Row(
                          spacing: 8,
                          children: [
                            Expanded(
                              child: _HTMLText(
                                post.title ?? '(제목 없음)',
                                style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w500),
                              ),
                            ),
                            Text(
                              post.post.updatedAt.fromNow(),
                              style: const TextStyle(fontSize: 14, color: AppColors.gray_700),
                            ),
                          ],
                        ),
                        _HTMLText(
                          post.text ?? '(내용 없음)',
                          style: const TextStyle(fontSize: 14, color: AppColors.gray_700),
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

class _HTMLText extends StatelessWidget {
  const _HTMLText(this.text, {this.style = const TextStyle(color: AppColors.gray_950)});

  final String text;
  final TextStyle style;

  TextSpan _buildTextSpan(String input) {
    final emRegExp = RegExp('<em>(.*?)</em>');
    final spans = <InlineSpan>[];
    var currentIndex = 0;

    for (final match in emRegExp.allMatches(input)) {
      if (match.start > currentIndex) {
        spans.add(TextSpan(text: input.substring(currentIndex, match.start), style: style));
      }

      spans.add(
        TextSpan(
          text: match.group(1),
          style: style.copyWith(fontWeight: FontWeight.w700, color: AppColors.gray_950),
        ),
      );

      currentIndex = match.end;
    }

    if (currentIndex < input.length) {
      spans.add(TextSpan(text: input.substring(currentIndex), style: style));
    }

    return TextSpan(children: spans);
  }

  @override
  Widget build(BuildContext context) {
    return Text.rich(_buildTextSpan(text), maxLines: 1, overflow: TextOverflow.ellipsis);
  }
}
