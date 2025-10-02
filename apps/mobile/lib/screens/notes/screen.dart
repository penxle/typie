import 'dart:async';
import 'dart:math';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:flutter_sticky_header/flutter_sticky_header.dart';
import 'package:gap/gap.dart';
import 'package:gql_tristate_value/gql_tristate_value.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/constants/router_tab_index.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/debounce.dart';
import 'package:typie/hooks/route_resumed.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/editor/values.dart';
import 'package:typie/screens/notes/__generated__/notes_create_note_mutation.req.gql.dart';
import 'package:typie/screens/notes/__generated__/notes_delete_note_mutation.req.gql.dart';
import 'package:typie/screens/notes/__generated__/notes_move_note_mutation.req.gql.dart';
import 'package:typie/screens/notes/__generated__/notes_query.data.gql.dart';
import 'package:typie/screens/notes/__generated__/notes_query.req.gql.dart';
import 'package:typie/screens/notes/__generated__/notes_update_note_mutation.req.gql.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/note.dart';
import 'package:typie/widgets/tappable.dart';

@RoutePage()
class NotesScreen extends HookWidget {
  const NotesScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final refreshNotifier = useMemoized(RefreshNotifier.new, []);

    useRouteResumed(context, refreshNotifier.refresh, tabIndex: RouteTabsIndex.notes);

    return GraphQLOperation(
      initialBackgroundColor: context.colors.surfaceSubtle,
      operation: GNotesScreen_QueryReq(),
      refreshNotifier: refreshNotifier,
      builder: (context, client, queryData) {
        final notes = queryData.notes.toList();
        final sortedNotes = List<GNotesScreen_QueryData_notes>.from(notes)..sort((a, b) => a.order.compareTo(b.order));

        return _NotesContent(sortedNotes: sortedNotes, refreshNotifier: refreshNotifier, queryData: queryData);
      },
    );
  }
}

class _NotesContent extends HookWidget {
  const _NotesContent({required this.sortedNotes, required this.refreshNotifier, required this.queryData});

  final List<GNotesScreen_QueryData_notes> sortedNotes;
  final RefreshNotifier refreshNotifier;
  final GNotesScreen_QueryData queryData;

  @override
  Widget build(BuildContext context) {
    final client = useService<GraphQLClient>();
    final mixpanel = useService<Mixpanel>();
    final debounce = useDebounce<void>(const Duration(milliseconds: 500));

    final noteControllers = useState<Map<String, TextEditingController>>({});
    final noteLocalUpdatedAt = useState<Map<String, DateTime>>({});
    final focusNodes = useState<Map<String, FocusNode>>({});
    final focusedNoteIdState = useState<String?>(null);
    final expandedNotes = useState<Set<String>>({});
    final pendingFocusNoteId = useRef<String?>(null);
    final selectedFilterEntityId = useState<String?>(null);
    final scrollController = useScrollController();

    useEffect(() {
      if (scrollController.hasClients) {
        scrollController.jumpTo(0);
      }
      return null;
    }, [selectedFilterEntityId.value]);

    useEffect(() {
      final newControllers = Map<String, TextEditingController>.from(noteControllers.value);
      final newFocusNodes = Map<String, FocusNode>.from(focusNodes.value);
      final newLocalUpdatedAt = Map<String, DateTime>.from(noteLocalUpdatedAt.value);

      for (final note in sortedNotes) {
        if (!newControllers.containsKey(note.id)) {
          newControllers[note.id] = TextEditingController(text: note.content);
          final focusNode = FocusNode();
          focusNode.addListener(() {
            if (focusNode.hasFocus) {
              focusedNoteIdState.value = note.id;
            } else if (focusedNoteIdState.value == note.id) {
              focusedNoteIdState.value = null;
            }
          });
          newFocusNodes[note.id] = focusNode;
        } else {
          final controller = newControllers[note.id]!;
          final updatedAt = DateTime.parse(note.updatedAt.toString());
          if (newLocalUpdatedAt[note.id] == null || updatedAt.isAfter(newLocalUpdatedAt[note.id]!)) {
            if (controller.text != note.content) {
              controller.text = note.content;
            }
            newLocalUpdatedAt[note.id] = updatedAt;
          }
        }
      }

      final currentIds = sortedNotes.map((n) => n.id).toSet();
      newControllers.removeWhere((id, controller) {
        if (!currentIds.contains(id)) {
          controller.dispose();
          return true;
        }
        return false;
      });
      newFocusNodes.removeWhere((id, focusNode) {
        if (!currentIds.contains(id)) {
          focusNode.dispose();
          return true;
        }
        return false;
      });
      debounce.timers().removeWhere((id, _) {
        if (!currentIds.contains(id)) {
          debounce.cancel(id);
          return true;
        }
        return false;
      });
      newLocalUpdatedAt.removeWhere((id, _) => !currentIds.contains(id));

      noteControllers.value = newControllers;
      focusNodes.value = newFocusNodes;
      noteLocalUpdatedAt.value = newLocalUpdatedAt;

      return null;
    }, [sortedNotes]);

    useEffect(() {
      return () {
        debounce.timers().keys.toList().forEach(debounce.cancel);
        for (final controller in noteControllers.value.values) {
          controller.dispose();
        }
        for (final focusNode in focusNodes.value.values) {
          focusNode.dispose();
        }
      };
    }, []);

    // NOTE: 노트 추가 후 포커스
    useEffect(() {
      if (pendingFocusNoteId.value != null && focusNodes.value.containsKey(pendingFocusNoteId.value)) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (scrollController.hasClients) {
            scrollController.jumpTo(0);
          }
          focusNodes.value[pendingFocusNoteId.value]?.requestFocus();
          pendingFocusNoteId.value = null;
        });
      }
      return null;
    }, [pendingFocusNoteId.value, focusNodes.value.keys.toList()]);

    Future<void> handleUpdateNoteContent(String noteId, String content) async {
      noteLocalUpdatedAt.value[noteId] = DateTime.now();

      debounce.call(() async {
        final request = GNotesScreen_UpdateNote_MutationReq(
          (b) => b
            ..vars.input.noteId = noteId
            ..vars.input.content = Value.present(content),
        );

        await client.request(request);
      }, noteId);
    }

    Future<void> handleUpdateNoteEntity(String noteId, String? entityId) async {
      final request = GNotesScreen_UpdateNote_MutationReq(
        (b) => b
          ..vars.input.noteId = noteId
          ..vars.input.entityId = Value.present(entityId),
      );
      await client.request(request);
      unawaited(mixpanel.track('update_note'));
      refreshNotifier.refresh();
    }

    Future<void> handleDeleteNote(String noteId) async {
      await context.showModal(
        child: ConfirmModal(
          title: '노트 삭제',
          message: '이 노트를 삭제하시겠어요?\n복구할 수 없어요.',
          confirmText: '삭제',
          confirmBackgroundColor: context.colors.accentDanger,
          onConfirm: () async {
            noteLocalUpdatedAt.value.remove(noteId);

            final request = GNotesScreen_DeleteNote_MutationReq((b) => b..vars.input.noteId = noteId);

            await client.request(request);

            unawaited(mixpanel.track('delete_note'));
            refreshNotifier.refresh();
          },
        ),
      );
    }

    List<Map<String, dynamic>> getNoteColors() {
      final backgroundColors = editorValues['textBackgroundColor']!;
      return backgroundColors.where((item) => item['value'] != 'none').toList();
    }

    String getRandomNoteColor() {
      final colors = getNoteColors();
      final random = Random();
      return colors[random.nextInt(colors.length)]['value'] as String;
    }

    Future<void> handleCreateNote() async {
      final request = GNotesScreen_CreateNote_MutationReq(
        (b) => b
          ..vars.input.content = ''
          ..vars.input.color = getRandomNoteColor()
          ..vars.input.entityId = Value.present(selectedFilterEntityId.value),
      );

      final response = await client.request(request);
      final newNoteId = response.createNote.id;

      expandedNotes.value = {...expandedNotes.value, newNoteId};
      pendingFocusNoteId.value = newNoteId;
      unawaited(mixpanel.track('create_note', properties: {'relatedToEntity': selectedFilterEntityId.value != null}));
      refreshNotifier.refresh();
    }

    void showEntitySelector({required String? currentEntityId, required void Function(String?) onSelectEntity}) {
      final recentEntities = queryData.me?.recentlyViewedEntities.take(10).toList() ?? [];

      unawaited(
        showModalBottomSheet<void>(
          context: context,
          isScrollControlled: true,
          backgroundColor: Colors.transparent,
          builder: (context) {
            return AppDraggableBottomSheet(
              builder: (context, scrollController) {
                return _EntitySelector(
                  recentEntities: recentEntities,
                  currentEntityId: currentEntityId,
                  scrollController: scrollController,
                  onSelectEntity: (entityId) {
                    onSelectEntity(entityId);
                    Navigator.pop(context);
                  },
                );
              },
            );
          },
        ),
      );
    }

    void showFilterEntitySelector() {
      showEntitySelector(
        currentEntityId: selectedFilterEntityId.value,
        onSelectEntity: (entityId) {
          selectedFilterEntityId.value = entityId;
        },
      );
    }

    void showRelatedEntitySelector(String noteId) {
      final currentEntityId = sortedNotes.firstWhere((n) => n.id == noteId).entity?.id;
      showEntitySelector(
        currentEntityId: currentEntityId,
        onSelectEntity: (entityId) {
          unawaited(handleUpdateNoteEntity(noteId, entityId));
        },
      );
    }

    final notesRelatedToEntity = selectedFilterEntityId.value != null
        ? sortedNotes.where((note) => note.entity?.id == selectedFilterEntityId.value).toList()
        : <GNotesScreen_QueryData_notes>[];
    final notesNotRelatedToEntity = selectedFilterEntityId.value != null
        ? sortedNotes.where((note) => note.entity?.id != selectedFilterEntityId.value).toList()
        : sortedNotes;

    GNotesScreen_QueryData_me_recentlyViewedEntities? selectedEntity;
    if (selectedFilterEntityId.value != null) {
      try {
        selectedEntity = queryData.me?.recentlyViewedEntities.firstWhere((e) => e.id == selectedFilterEntityId.value);
      } catch (_) {
        selectedEntity = null;
      }
    }

    String? getSelectedEntityTitle() {
      if (selectedEntity == null) {
        return null;
      }
      final node = selectedEntity.node;
      if (node.G__typename == 'Post') {
        return (node as GNotesScreen_QueryData_me_recentlyViewedEntities_node__asPost).title;
      } else if (node.G__typename == 'Canvas') {
        return (node as GNotesScreen_QueryData_me_recentlyViewedEntities_node__asCanvas).title;
      }
      return null;
    }

    void handleExpand(String noteId) {
      expandedNotes.value = {...expandedNotes.value, noteId};
      WidgetsBinding.instance.addPostFrameCallback((_) {
        focusNodes.value[noteId]?.requestFocus();
        final controller = noteControllers.value[noteId];
        if (controller != null) {
          controller.selection = TextSelection.fromPosition(TextPosition(offset: controller.text.length));
        }
      });
    }

    void handleCollapse(String noteId) {
      expandedNotes.value = {...expandedNotes.value}..remove(noteId);
      focusNodes.value[noteId]?.unfocus();
    }

    Future<void> handleReorder(List<GNotesScreen_QueryData_notes> notesList, int oldIndex, int newIndex) async {
      var adjustedNewIndex = newIndex;
      if (oldIndex < newIndex) {
        adjustedNewIndex -= 1;
      }

      final movedNoteId = notesList[oldIndex].id;
      final movedNote = notesList.removeAt(oldIndex);
      notesList.insert(adjustedNewIndex, movedNote);

      final lowerNote = adjustedNewIndex > 0 ? notesList[adjustedNewIndex - 1] : null;
      final upperNote = adjustedNewIndex < notesList.length - 1 ? notesList[adjustedNewIndex + 1] : null;

      final request = GNotesScreen_MoveNote_MutationReq(
        (b) => b
          ..vars.input.noteId = movedNoteId
          ..vars.input.lowerOrder = Value.present(lowerNote?.order)
          ..vars.input.upperOrder = Value.present(upperNote?.order),
      );

      await client.request(request);
      unawaited(mixpanel.track('move_note'));
      refreshNotifier.refresh();
    }

    Future<void> handleNavigateToEntity(String noteId, List<GNotesScreen_QueryData_notes> notesList) async {
      final note = notesList.firstWhere((n) => n.id == noteId);
      if (note.entity?.slug != null) {
        await context.router.push(EditorRoute(slug: note.entity!.slug));
      }
    }

    return Scaffold(
      backgroundColor: context.colors.surfaceDefault,
      resizeToAvoidBottomInset: false,
      appBar: PreferredSize(
        preferredSize: const Size.fromHeight(56),
        child: Heading(
          title: '노트',
          titleIcon: LucideLightIcons.sticky_note,
          actions: [HeadingAction(icon: LucideLightIcons.plus, onTap: handleCreateNote)],
        ),
      ),
      body: sortedNotes.isEmpty
          ? const _EmptyNotesView()
          : CustomScrollView(
              controller: scrollController,
              slivers: [
                if (selectedFilterEntityId.value != null)
                  SliverStickyHeader(
                    header: _NotesSectionHeader(
                      title: '${getSelectedEntityTitle() ?? "선택된 항목"} 관련 노트',
                      showSelector: true,
                      onTap: showFilterEntitySelector,
                    ),
                    sliver: notesRelatedToEntity.isEmpty
                        ? SliverToBoxAdapter(
                            child: Padding(
                              padding: const Pad(horizontal: 20),
                              child: Container(
                                width: double.infinity,
                                padding: const Pad(vertical: 24, horizontal: 20),
                                decoration: BoxDecoration(
                                  color: context.colors.surfaceSubtle,
                                  borderRadius: BorderRadius.circular(8),
                                ),
                                child: Text(
                                  '이 포스트 관련 노트가 없어요',
                                  style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
                                  textAlign: TextAlign.center,
                                ),
                              ),
                            ),
                          )
                        : _NotesReorderableList(
                            notes: notesRelatedToEntity,
                            noteControllers: noteControllers.value,
                            focusNodes: focusNodes.value,
                            expandedNotes: expandedNotes.value,
                            onExpand: handleExpand,
                            onCollapse: handleCollapse,
                            onDelete: handleDeleteNote,
                            onUpdateContent: (noteId, value) {
                              unawaited(handleUpdateNoteContent(noteId, value));
                            },
                            onSelectEntity: showRelatedEntitySelector,
                            onNavigateToEntity: (noteId) async {
                              await handleNavigateToEntity(noteId, notesRelatedToEntity);
                            },
                            onReorder: (oldIndex, newIndex) async {
                              await handleReorder(notesRelatedToEntity, oldIndex, newIndex);
                            },
                          ),
                  ),
                if (selectedFilterEntityId.value != null && notesNotRelatedToEntity.isNotEmpty)
                  const SliverToBoxAdapter(child: SizedBox(height: 24)),
                if (selectedFilterEntityId.value == null || notesNotRelatedToEntity.isNotEmpty)
                  SliverStickyHeader(
                    header: _NotesSectionHeader(
                      title: '모든 노트',
                      showSelector: selectedFilterEntityId.value == null,
                      onTap: showFilterEntitySelector,
                    ),
                    sliver: _NotesReorderableList(
                      notes: notesNotRelatedToEntity,
                      noteControllers: noteControllers.value,
                      focusNodes: focusNodes.value,
                      expandedNotes: expandedNotes.value,
                      onExpand: handleExpand,
                      onCollapse: handleCollapse,
                      onDelete: handleDeleteNote,
                      onUpdateContent: (noteId, value) {
                        unawaited(handleUpdateNoteContent(noteId, value));
                      },
                      onSelectEntity: showRelatedEntitySelector,
                      onNavigateToEntity: (noteId) async {
                        await handleNavigateToEntity(noteId, notesNotRelatedToEntity);
                      },
                      onReorder: (oldIndex, newIndex) async {
                        await handleReorder(notesNotRelatedToEntity, oldIndex, newIndex);
                      },
                    ),
                  ),
                SliverPadding(padding: EdgeInsets.only(bottom: MediaQuery.viewPaddingOf(context).bottom + 20)),
              ],
            ),
    );
  }
}

class _NotesReorderableList extends StatelessWidget {
  const _NotesReorderableList({
    required this.notes,
    required this.noteControllers,
    required this.focusNodes,
    required this.expandedNotes,
    required this.onExpand,
    required this.onCollapse,
    required this.onDelete,
    required this.onUpdateContent,
    required this.onSelectEntity,
    required this.onNavigateToEntity,
    required this.onReorder,
  });

  final List<GNotesScreen_QueryData_notes> notes;
  final Map<String, TextEditingController?> noteControllers;
  final Map<String, FocusNode?> focusNodes;
  final Set<String> expandedNotes;
  final void Function(String noteId) onExpand;
  final void Function(String noteId) onCollapse;
  final void Function(String noteId) onDelete;
  final void Function(String noteId, String value) onUpdateContent;
  final void Function(String noteId) onSelectEntity;
  final void Function(String noteId) onNavigateToEntity;
  final Future<void> Function(int oldIndex, int newIndex) onReorder;

  @override
  Widget build(BuildContext context) {
    return SliverReorderableList(
      itemCount: notes.length,
      onReorder: onReorder,
      onReorderStart: (index) async {
        await HapticFeedback.lightImpact();
      },
      onReorderEnd: (index) async {
        await HapticFeedback.lightImpact();
      },
      proxyDecorator: (child, index, animation) => child,
      itemBuilder: (context, index) {
        final note = notes[index];
        final controller = noteControllers[note.id];
        final focusNode = focusNodes[note.id];
        final isExpanded = expandedNotes.contains(note.id);

        String? getEntityTitle() {
          if (note.entity == null) {
            return null;
          }
          final node = note.entity!.node;
          if (node.G__typename == 'Post') {
            return (node as GNotesScreen_QueryData_notes_entity_node__asPost).title;
          } else if (node.G__typename == 'Canvas') {
            return (node as GNotesScreen_QueryData_notes_entity_node__asCanvas).title;
          }
          return null;
        }

        IconData? getEntityIcon() {
          if (note.entity == null) {
            return null;
          }
          final node = note.entity!.node;
          if (node.G__typename == 'Post') {
            final postNode = node as GNotesScreen_QueryData_notes_entity_node__asPost;
            return postNode.type == GPostType.TEMPLATE ? LucideLightIcons.shapes : LucideLightIcons.file;
          } else if (node.G__typename == 'Canvas') {
            return LucideLightIcons.line_squiggle;
          }
          return null;
        }

        final entityTitle = getEntityTitle();
        final entityIcon = getEntityIcon();

        return Padding(
          key: ValueKey(note.id),
          padding: Pad(horizontal: 20, bottom: index == notes.length - 1 ? 0 : 12),
          child: NoteCard(
            color: note.color,
            index: index,
            controller: controller,
            focusNode: focusNode,
            isExpanded: isExpanded,
            onExpand: () => onExpand(note.id),
            onUpdateContent: (value) => onUpdateContent(note.id, value),
            footer: NoteFooter(
              entity: entityTitle != null && entityIcon != null
                  ? NoteFooterEntity(
                      entityTitle: entityTitle,
                      entityIcon: entityIcon,
                      isExpanded: isExpanded,
                      onSelectEntity: () => onSelectEntity(note.id),
                      onNavigateToEntity: () => onNavigateToEntity(note.id),
                    )
                  : null,
              emptyEntity: entityTitle == null || entityIcon == null
                  ? NoteFooterEmptyEntity(onSelectEntity: () => onSelectEntity(note.id))
                  : null,
              isExpanded: isExpanded,
              onDelete: () => onDelete(note.id),
              onCollapse: () => onCollapse(note.id),
              onExpand: () => onExpand(note.id),
            ),
          ),
        );
      },
    );
  }
}

class _EmptyNotesView extends StatelessWidget {
  const _EmptyNotesView();

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Container(
            width: 64,
            height: 64,
            decoration: BoxDecoration(borderRadius: BorderRadius.circular(16), color: context.colors.surfaceMuted),
            child: Icon(LucideLightIcons.sticky_note, color: context.colors.textFaint, size: 28),
          ),
          const SizedBox(height: 20),
          Text(
            '아직 작성한 노트가 없어요',
            textAlign: TextAlign.center,
            style: TextStyle(fontSize: 16, color: context.colors.textFaint),
          ),
        ],
      ),
    );
  }
}

class _EntitySelector extends StatelessWidget {
  const _EntitySelector({
    required this.recentEntities,
    required this.currentEntityId,
    required this.scrollController,
    required this.onSelectEntity,
  });

  final List<GNotesScreen_QueryData_me_recentlyViewedEntities> recentEntities;
  final String? currentEntityId;
  final ScrollController scrollController;
  final void Function(String?) onSelectEntity;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const Pad(horizontal: 20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            '관련 항목 선택',
            style: TextStyle(fontSize: 17, fontWeight: FontWeight.w700, color: context.colors.textSubtle),
          ),
          const Gap(12),
          Expanded(
            child: ListView(
              controller: scrollController,
              padding: EdgeInsets.only(bottom: MediaQuery.of(context).padding.bottom + 20),
              children: [
                _EntitySelectorItem(
                  title: '없음',
                  icon: null,
                  isSelected: currentEntityId == null,
                  onTap: () => onSelectEntity(null),
                ),
                ...recentEntities.map((entity) {
                  final node = entity.node;
                  String? title;
                  IconData? icon;

                  if (node.G__typename == 'Post') {
                    final postNode = node as GNotesScreen_QueryData_me_recentlyViewedEntities_node__asPost;
                    title = postNode.title;
                    icon = postNode.type == GPostType.TEMPLATE ? LucideLightIcons.shapes : LucideLightIcons.file;
                  } else if (node.G__typename == 'Canvas') {
                    final canvasNode = node as GNotesScreen_QueryData_me_recentlyViewedEntities_node__asCanvas;
                    title = canvasNode.title;
                    icon = LucideLightIcons.line_squiggle;
                  }

                  if (title == null || icon == null) {
                    return const SizedBox.shrink();
                  }

                  return Padding(
                    padding: const Pad(top: 8),
                    child: _EntitySelectorItem(
                      title: title,
                      icon: icon,
                      isSelected: currentEntityId == entity.id,
                      onTap: () => onSelectEntity(entity.id),
                    ),
                  );
                }),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _EntitySelectorItem extends StatelessWidget {
  const _EntitySelectorItem({required this.title, required this.icon, required this.isSelected, required this.onTap});

  final String title;
  final IconData? icon;
  final bool isSelected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      onTap: onTap,
      child: Container(
        decoration: BoxDecoration(
          border: Border.all(color: isSelected ? context.colors.borderInverse : context.colors.borderDefault),
          borderRadius: BorderRadius.circular(8),
        ),
        padding: const Pad(horizontal: 12, vertical: 12),
        child: Row(
          children: [
            if (icon != null) ...[Icon(icon, size: 18, color: context.colors.textSubtle), const SizedBox(width: 8)],
            Expanded(
              child: Text(title, style: const TextStyle(fontSize: 16), maxLines: 1, overflow: TextOverflow.ellipsis),
            ),
            if (isSelected) Icon(LucideLightIcons.check, color: context.colors.textDefault, size: 20),
          ],
        ),
      ),
    );
  }
}

class _NotesSectionHeader extends StatelessWidget {
  const _NotesSectionHeader({required this.title, required this.showSelector, required this.onTap});

  final String title;
  final bool showSelector;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final textWidget = Text(
      title,
      style: TextStyle(fontSize: 15, fontWeight: FontWeight.w500, color: context.colors.textFaint),
      overflow: TextOverflow.ellipsis,
      maxLines: 1,
    );

    if (!showSelector) {
      return Container(
        color: context.colors.surfaceDefault,
        padding: const Pad(horizontal: 20, vertical: 8),
        child: textWidget,
      );
    }

    return Container(
      color: context.colors.surfaceDefault,
      padding: const Pad(horizontal: 20, vertical: 8),
      child: Tappable(
        onTap: onTap,
        child: SizedBox(
          width: double.infinity,
          child: Row(
            children: [
              Flexible(child: textWidget),
              const Gap(4),
              Icon(LucideLightIcons.chevron_down, size: 16, color: context.colors.textFaint),
            ],
          ),
        ),
      ),
    );
  }
}
