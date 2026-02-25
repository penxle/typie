#ifndef EDITOR_H
#define EDITOR_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct EditorApplication EditorApplication;
typedef struct EditorHandle EditorHandle;

typedef struct {
    uint32_t width;
    uint32_t height;
    size_t buffer_size;
} RenderInfo;

typedef struct {
    uint32_t doc_with_whitespace;
    uint32_t doc_without_whitespace;
    uint32_t doc_without_whitespace_and_punctuation;
    uint32_t selection_with_whitespace;
    uint32_t selection_without_whitespace;
    uint32_t selection_without_whitespace_and_punctuation;
} CharacterCounts;

typedef void (*LogCallback)(int32_t level, const char* message);

uint8_t* editor_alloc(size_t size);
void editor_free(uint8_t* ptr, size_t len, size_t capacity);
void editor_free_string(char* ptr);

char* editor_get_last_error(void);
void editor_clear_last_error(void);

void editor_set_log_callback(LogCallback callback);

int32_t editor_validate_regex(const char* pattern);

EditorApplication* editor_application_new(void);
void editor_application_free(EditorApplication* app);
int32_t editor_application_load_icu_data(EditorApplication* app, const uint8_t* data, size_t len);
int32_t editor_application_add_font_base(EditorApplication* app, const char* family, uint16_t weight, const uint8_t* data, size_t data_len);
int32_t editor_application_add_font_chunk(EditorApplication* app, const char* family, uint16_t weight, const uint8_t* data, size_t data_len);
int32_t editor_application_set_available_fonts(EditorApplication* app, const char* fonts_json);
int32_t editor_application_set_fallback_fonts(EditorApplication* app, const char* names_json);
int32_t editor_application_set_text_replacement_rules(EditorApplication* app, const char* rules_json);
int32_t editor_application_clear_text_replacement_rules(EditorApplication* app);
EditorHandle* editor_application_create_editor(EditorApplication* app, double scale_factor, const uint8_t* snapshot, size_t snapshot_len);

void editor_handle_free(EditorHandle* editor);
int32_t editor_dispatch(EditorHandle* editor, const char* message_json);
int32_t editor_tick(EditorHandle* editor);
const uint8_t* editor_get_slate_ptr(EditorHandle* editor);
uint32_t editor_get_slate_len(EditorHandle* editor);
const uint8_t* editor_get_slab_ptr(EditorHandle* editor);
uint32_t editor_get_slab_len(EditorHandle* editor);
char* editor_get_slate_offsets(void);
int32_t editor_flush(EditorHandle* editor);
size_t editor_get_page_count(EditorHandle* editor);
int32_t editor_get_render_info(EditorHandle* editor, size_t page_index, RenderInfo* out_info);

#define PIXEL_FORMAT_RGBA 0
#define PIXEL_FORMAT_BGRA 1
int32_t editor_render_page_to(
    EditorHandle* editor,
    size_t page_index,
    uint8_t* dst,
    size_t dst_stride,
    size_t dst_width,
    size_t dst_height,
    int32_t format
);
int32_t editor_is_selection_hit(EditorHandle* editor, size_t page_idx, float x, float y);
int32_t editor_is_interactive_hit(EditorHandle* editor, size_t page_idx, float x, float y);
int32_t editor_get_character_counts(EditorHandle* editor, CharacterCounts* out_counts);

typedef struct {
    uint32_t width;
    uint32_t height;
    float offset_x;
    float offset_y;
    float scale_factor;
    uint8_t* pixels;
    size_t len;
} DragImageResult;

int32_t editor_render_drag_image(
    EditorHandle* editor,
    const size_t* visible_pages,
    size_t visible_pages_len,
    size_t page_idx,
    DragImageResult* out_result
);

#define EDITOR_EXPORT_SNAPSHOT 0
#define EDITOR_EXPORT_VERSION 1
#define EDITOR_EXPORT_ALL_UPDATES 2
#define EDITOR_EXPORT_UPDATES_FROM 3
uint8_t* editor_export(EditorHandle* editor, int32_t mode, const uint8_t* version, size_t version_len, size_t* out_len);
int32_t editor_import_updates(EditorHandle* editor, const uint8_t* updates, size_t len);
int32_t editor_import_updates_batch(
    EditorHandle* editor,
    const uint8_t* const* updates_ptrs,
    const size_t* updates_lens,
    size_t count
);
char* editor_get_clipboard_data(EditorHandle* editor);

char* editor_get_text_with_mappings(EditorHandle* editor);
int32_t editor_set_tracked_items(EditorHandle* editor, uint32_t group, const char* items_json);
int32_t editor_remove_tracked_items(EditorHandle* editor, uint32_t group, const char* ids_json);
char* editor_perform_search(EditorHandle* editor, const char* query, int32_t match_whole_word);
int32_t editor_replace_text_in_block(
    EditorHandle* editor, const char* block_id,
    size_t start_offset, size_t end_offset, const char* replacement
);
int32_t editor_replace_text_in_blocks(EditorHandle* editor, const char* items_json);

int32_t editor_insert_template_fragment(EditorHandle* editor, const uint8_t* snapshot, size_t snapshot_len);

#ifdef __cplusplus
}
#endif

#endif
