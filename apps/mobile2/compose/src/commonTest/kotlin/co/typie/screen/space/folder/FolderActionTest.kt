package co.typie.screen.space.folder

import kotlin.test.Test
import kotlin.test.assertEquals

class FolderActionTest {
  @Test
  fun `folder primary action sections match center popover order`() {
    assertEquals(
      listOf(
        listOf("이름 변경", "아이콘 변경"),
        listOf("스페이스에서 열기", "공유 및 게시"),
        listOf("다른 폴더로 옮기기", "복사", "잘라내기"),
        listOf("삭제"),
      ),
      folderPrimaryActionSections().map { section -> section.items.map { item -> item.label } },
    )
  }
}
