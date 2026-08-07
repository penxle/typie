package co.typie.screen.editor.editor.attachment

import kotlin.test.Test
import kotlin.test.assertEquals

class EditorAttachmentDownloadTest {
  @Test
  fun `decodes the API utf8 filename`() {
    assertEquals(
      "타이피 이미지.svg",
      parseAttachmentFilename(
        "inline; filename*=UTF-8''%ED%83%80%EC%9D%B4%ED%94%BC%20%EC%9D%B4%EB%AF%B8%EC%A7%80.svg"
      ),
    )
  }
}
