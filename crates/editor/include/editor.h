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
    uint8_t* ptr;
    size_t len;
    uint32_t width;
    uint32_t height;
} RenderResult;

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
int32_t editor_render_page(EditorHandle* editor, size_t page_index, RenderResult* out_result);

#ifdef __cplusplus
}
#endif

#endif
