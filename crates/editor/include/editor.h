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

EditorApplication* editor_application_new(void);
void editor_application_free(EditorApplication* app);
int32_t editor_application_load_icu_data(EditorApplication* app, const uint8_t* data, size_t len);
int32_t editor_application_add_font(EditorApplication* app, const char* name, uint16_t weight, const uint8_t* data, size_t data_len);
int32_t editor_application_register_fallback_font(EditorApplication* app, const char* name);
int32_t editor_application_set_available_fonts(EditorApplication* app, const char* fonts_json);
EditorHandle* editor_application_create_editor(EditorApplication* app, double scale_factor, const uint8_t* snapshot, size_t snapshot_len);

void editor_handle_free(EditorHandle* editor);
int32_t editor_dispatch(EditorHandle* editor, const char* message_json);
char* editor_tick(EditorHandle* editor);
void editor_flush(EditorHandle* editor);
size_t editor_get_page_count(EditorHandle* editor);
int32_t editor_get_render_info(EditorHandle* editor, size_t page_index, RenderInfo* out_info);

#define PIXEL_FORMAT_RGBA 0
#define PIXEL_FORMAT_BGRA 1
int32_t editor_render_page_to(EditorHandle* editor, size_t page_index, uint8_t* dst, size_t dst_stride, size_t dst_height, int32_t format);
int32_t editor_can_drag_at(EditorHandle* editor, size_t page_idx, float x, float y);
int32_t editor_get_character_counts(EditorHandle* editor, CharacterCounts* out_counts);

uint8_t* editor_get_snapshot(EditorHandle* editor, size_t* out_len);
uint8_t* editor_get_version(EditorHandle* editor, size_t* out_len);
uint8_t* editor_export_all_updates(EditorHandle* editor, size_t* out_len);
int32_t editor_export_new_updates(
    EditorHandle* editor,
    uint8_t** out_updates,
    size_t* out_updates_len,
    uint8_t** out_version,
    size_t* out_version_len
);
int32_t editor_import_updates(EditorHandle* editor, const uint8_t* updates, size_t len);
int32_t editor_import_updates_batch(
    EditorHandle* editor,
    const uint8_t* const* updates_ptrs,
    const size_t* updates_lens,
    size_t count
);
int32_t editor_commit_sync(EditorHandle* editor, const uint8_t* version, size_t version_len);
char* editor_get_clipboard_data(EditorHandle* editor);

char* editor_get_spellcheck_text(EditorHandle* editor);
int32_t editor_set_spellcheck_errors(EditorHandle* editor, const char* errors_json);
int32_t editor_apply_spellcheck_correction(
    EditorHandle* editor, const char* block_id,
    size_t start_offset, size_t end_offset, const char* correction
);
int32_t editor_clear_spellcheck_errors(EditorHandle* editor);

#ifdef __cplusplus
}
#endif

#endif
