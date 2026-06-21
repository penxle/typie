package co.typie.domain.note

import co.typie.graphql.type.NoteStatus

internal fun NoteStatus.filterLabel(): String =
  when (this) {
    NoteStatus.OPEN -> "진행 중"
    NoteStatus.RESOLVED -> "완료됨"
    NoteStatus.UNKNOWN__ -> "진행 중"
  }

internal fun NoteStatus.emptyMessage(): String =
  when (this) {
    NoteStatus.OPEN -> "진행 중 노트가 없어요"
    NoteStatus.RESOLVED -> "완료된 노트가 없어요"
    NoteStatus.UNKNOWN__ -> "진행 중 노트가 없어요"
  }

internal fun NoteStatus.toggled(): NoteStatus =
  when (this) {
    NoteStatus.OPEN -> NoteStatus.RESOLVED
    NoteStatus.RESOLVED -> NoteStatus.OPEN
    NoteStatus.UNKNOWN__ -> NoteStatus.OPEN
  }
