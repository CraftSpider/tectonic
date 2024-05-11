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

#define BIGGEST_CHAR 65535

#define BIGGEST_USV 1114111

#define DIMEN_VAL_LIMIT 256

#define NATIVE_NODE_SIZE 6

#define INT_BASE 7826729

#define INT_PAR__new_line_char 49

#define INT_PAR__escape_char 45

#define NATIVE_INFO_OFFSET 4

#define OTGR_FONT_FLAG 65534

#define PRIM_SIZE 2100

#define ACTIVE_BASE 1

#define SINGLE_BASE 1114113

#define NULL_CS 2228225

#define HASH_BASE 2228226

#define PRIM_EQTB_BASE 2254339

#define FROZEN_NULL_FONT 2243238

#define UNDEFINED_CONTROL_SEQUENCE 2254339

#define CAT_CODE_BASE 2256169

#define EQTB_SIZE 8941458

#define LETTER 11

#define TEXT_SIZE 0

#define SCRIPT_SIZE 256

#define SCRIPT_SCRIPT_SIZE 512

#if defined(WORDS_BIGENDIAN)
typedef struct {
  int32_t s1;
  int32_t s0;
} b32x2;
#endif

#if !defined(WORDS_BIGENDIAN)
typedef struct {
  int32_t s0;
  int32_t s1;
} b32x2;
#endif

#if defined(WORDS_BIGENDIAN)
typedef struct {
  uint16_t s3;
  uint16_t s2;
  uint16_t s1;
  uint16_t s0;
} b16x4;
#endif

#if !defined(WORDS_BIGENDIAN)
typedef struct {
  uint16_t s0;
  uint16_t s1;
  uint16_t s2;
  uint16_t s3;
} b16x4;
#endif

typedef union {
  b32x2 b32;
  b16x4 b16;
  double gr;
  void *ptr;
} memory_word;

typedef int32_t scaled_t;

typedef unsigned short UTF16Code;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

uint16_t native_glyph_count(memory_word *node);

void set_native_glyph_count(memory_word *node, uint16_t val);

void print_utf8_str(const uint8_t *str, int len);

void print_chars(const unsigned short *str, int len);

void check_for_tfm_font_mapping(void);

void linebreak_start(int f, int32_t locale_str_num, uint16_t *text, int32_t text_len);

int linebreak_next(int f);

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

void ot_get_font_metrics(void *engine,
                         scaled_t *ascent,
                         scaled_t *descent,
                         scaled_t *xheight,
                         scaled_t *capheight,
                         scaled_t *slant);

XeTeXLayoutEngine loadOTfont(RawPlatformFontRef,
                             XeTeXFont font,
                             Fixed scaled_size,
                             const char *cp1);

void *load_tfm_font_mapping(void);

int apply_tfm_font_mapping(void *cnv, int c);

float glyph_height(int f, int g);

float glyph_depth(int f, int g);

extern char *gettexstring(int32_t s);

extern int32_t maketexstring(const char *s);

void capture_to_diagnostic(ttbc_diagnostic_t *diagnostic);

void diagnostic_print_file_line(ttbc_diagnostic_t *diagnostic);

ttbc_diagnostic_t *diagnostic_begin_capture_warning_here(void);

ttbc_diagnostic_t *error_here_with_diagnostic(const char *message);

void warn_char(int c);

void print_ln(void);

void print_raw_char(UTF16Code s, bool incr_offset);

void print_char(int32_t s);

void print(int32_t s);

void print_cstr(const char *str);

void print_nl(int32_t s);

void print_nl_cstr(const char *str);

void print_esc(int32_t s);

void print_esc_cstr(const char *str);

void print_the_digs(uint8_t k);

void print_int(int32_t n);

void print_cs(int32_t p);

void sprint_cs(int32_t p);

void print_file_name(int32_t n, int32_t a, int32_t e);

void print_size(int32_t s);

void print_write_whatsit(const char *s, int32_t p);

void print_native_word(int32_t p);

void print_sa_num(int32_t q);

int32_t tex_round(double r);

int32_t half(int32_t x);

scaled_t mult_and_add(int32_t n, scaled_t x, scaled_t y, scaled_t max_answer);

scaled_t x_over_n(scaled_t x, int32_t n);

scaled_t xn_over_d(scaled_t x, int32_t n, int32_t d);

scaled_t round_xn_over_d(scaled_t x, int32_t n, int32_t d);

void init_randoms(int32_t seed);

int32_t unif_rand(int32_t x);

int32_t norm_rand(void);

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus

#endif /* TECTONIC_ENGINE_XETEX_BINDGEN_H */
