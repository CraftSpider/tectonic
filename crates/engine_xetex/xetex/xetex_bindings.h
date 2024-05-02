#ifndef TECTONIC_ENGINE_XETEX_BINDGEN_H
#define TECTONIC_ENGINE_XETEX_BINDGEN_H

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * A serial number describing the detailed binary layout of the TeX "format
 * files" used by this crate. This number will occasionally increment,
 * indicating that the format file structure has changed. There is no provision
 * for partial forwards or backwards compatibility: if the number changes, you
 * need to regenerate your format files. If you’re generating format files, you
 * should munge this serial number in the filename, or something along those
 * lines, to make sure that when the engine is updated you don’t attempt to
 * reuse old files.
 */
#define FORMAT_SERIAL 33

#define FONT_FLAGS_COLORED 1

#define FONT_FLAGS_VERTICAL 2

#define AUTO 0

#define UTF8 1

#define UTF16BE 2

#define UTF16LE 3

#define RAW 4

#define ICUMAPPING 5

#define NATIVE_INFO_OFFSET 4

#define OTGR_FONT_FLAG 65534

typedef int32_t scaled_t;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

extern void font_mapping_warning(const void *mappingNameP,
                                 int32_t mappingNameLen,
                                 int32_t warningType);

extern void begin_diagnostic(void);

extern void print_nl(int32_t s);

extern void print_char(int32_t c);

extern void print_int(int32_t n);

extern void end_diagnostic(bool blank_line);

extern void font_feature_warning(const void *featureNameP,
                                 int32_t featLen,
                                 const void *settingNameP,
                                 int32_t setLen);

void linebreak_start(int f, int32_t locale_str_num, uint16_t *text, int32_t text_len);

int linebreak_next(XeTeXLayoutEngine engine);

int get_encoding_mode_and_info(int32_t *info);

double read_double(const char **s);

/**
 * returns 1 to go to next_option, -1 for bad_option, 0 to continue
 */
int readCommonFeatures(const char *feat,
                       const char *end,
                       float *extend,
                       float *slant,
                       float *embolden,
                       float *letterspace,
                       uint32_t *rgb_value);

void splitFontName(const char *name,
                   const char **var,
                   const char **feat,
                   const char **end,
                   int *index);

void ot_get_font_metrics(XeTeXLayoutEngine engine,
                         scaled_t *ascent,
                         scaled_t *descent,
                         scaled_t *xheight,
                         scaled_t *capheight,
                         scaled_t *slant);

XeTeXLayoutEngine loadOTfont(RawPlatformFontRef,
                             XeTeXFont font,
                             Fixed scaled_size,
                             const char *cp1);

extern void *load_mapping_file(const char *s, const char *e, char byteMapping);

extern char *gettexstring(int32_t s);

extern int32_t maketexstring(const char *s);

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus

#endif /* TECTONIC_ENGINE_XETEX_BINDGEN_H */
