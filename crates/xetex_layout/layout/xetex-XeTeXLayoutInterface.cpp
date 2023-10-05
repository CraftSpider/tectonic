/****************************************************************************\
 Part of the XeTeX typesetting system
 Copyright (c) 1994-2008 by SIL International
 Copyright (c) 2009-2012 by Jonathan Kew
 Copyright (c) 2012-2015 by Khaled Hosny

 SIL Author(s): Jonathan Kew

Permission is hereby granted, free of charge, to any person obtaining
a copy of this software and associated documentation files (the
"Software"), to deal in the Software without restriction, including
without limitation the rights to use, copy, modify, merge, publish,
distribute, sublicense, and/or sell copies of the Software, and to
permit persons to whom the Software is furnished to do so, subject to
the following conditions:

The above copyright notice and this permission notice shall be
included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT. IN NO EVENT SHALL THE COPYRIGHT HOLDERS BE LIABLE
FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF
CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

Except as contained in this notice, the name of the copyright holders
shall not be used in advertising or otherwise to promote the sale,
use or other dealings in this Software without prior written
authorization from the copyright holders.
\****************************************************************************/

#include "tectonic_bridge_core.h"

#include <unicode/ubidi.h>  /* Barely needed in this file. */

#include <graphite2/Font.h>
#include <graphite2/Segment.h>
#include <harfbuzz/hb.h>
#include <harfbuzz/hb-graphite2.h>
#if !HB_VERSION_ATLEAST(2,5,0)
/* Note: this configuration is no longer actively tested */
#include <harfbuzz/hb-icu.h>
#endif
#include <harfbuzz/hb-ot.h>

#include "tectonic_xetex_layout.h"

#include "xetex-XeTeXFontInst.h"
#ifdef XETEX_MAC
#include "xetex-XeTeXFontInst_Mac.h"
#endif
#include "xetex-XeTeXFontMgr.h"

struct XeTeXLayoutEngine_rec
{
    XeTeXFontInst*  font;
    PlatformFontRef fontRef;
    hb_tag_t        script;
    hb_language_t   language;
    hb_feature_t*   features;
    char**          ShaperList; // the requested shapers
    bool            shaperListToFree;
    char*           shaper;     // the actually used shaper
    int             nFeatures;
    uint32_t        rgbValue;
    float           extend;
    float           slant;
    float           embolden;
    hb_buffer_t*    hbBuffer;
};

/*******************************************************************/
/* Glyph bounding box cache to speed up \XeTeXuseglyphmetrics mode */
/*******************************************************************/

/*******************************************************************/

void
terminate_font_manager()
{
    XeTeXFontMgr::Terminate();
}

void
destroy_font_manager()
{
    XeTeXFontMgr::Destroy();
}

PlatformFontRef
findFontByName(const char* name, char* var, double size)
{
    return XeTeXFontMgr::GetFontManager()->findFont(name, var, size);
}

char
getReqEngine()
{
    return XeTeXFontMgr::GetFontManager()->getReqEngine();
}

void
setReqEngine(char reqEngine)
{
    XeTeXFontMgr::GetFontManager()->setReqEngine(reqEngine);
}

const char*
getFullName(PlatformFontRef fontRef)
{
    return XeTeXFontMgr::GetFontManager()->getFullName(fontRef);
}

double
getDesignSize(XeTeXFont font)
{
    return XeTeXFontMgr::GetFontManager()->getDesignSize(font);
}

char*
getFontFilename(XeTeXLayoutEngine engine, uint32_t* index)
{
    return xstrdup(engine->font->getFilename(index));
}

PlatformFontRef
getFontRef(XeTeXLayoutEngine engine)
{
    return engine->fontRef;
}

hb_tag_t
getIndScript(XeTeXFont font, unsigned int index)
{
    hb_tag_t rval = 0;

    hb_tag_t* scriptList;

    unsigned int scriptCount = getLargerScriptListTable(font, &scriptList);
    if (scriptList != NULL) {
        if (index < scriptCount)
            rval = scriptList[index];
    }

    return rval;
}

hb_tag_t
getIndLanguage(XeTeXFont font, hb_tag_t script, unsigned int index)
{
    hb_tag_t rval = 0;

    hb_face_t* face = hb_font_get_face(((XeTeXFontInst*)font)->getHbFont());
    hb_tag_t* scriptList;

    unsigned int scriptCount = getLargerScriptListTable(font, &scriptList);
    if (scriptList != NULL) {
        for (unsigned int i = 0; i < scriptCount; i++) {
            if (scriptList[i] == script) {
                unsigned int langCount;
                hb_tag_t* langList;

                langCount = hb_ot_layout_script_get_language_tags(face, HB_OT_TAG_GSUB, i, 0, NULL, NULL);
                langList = (hb_tag_t*) xcalloc(langCount, sizeof(hb_tag_t*));
                hb_ot_layout_script_get_language_tags(face, HB_OT_TAG_GSUB, i, 0, &langCount, langList);

                if (index < langCount) {
                    rval = langList[index];
                    break;
                }

                free(langList);

                langCount = hb_ot_layout_script_get_language_tags(face, HB_OT_TAG_GPOS, i, 0, NULL, NULL);
                langList = (hb_tag_t*) xcalloc(langCount, sizeof(hb_tag_t*));
                hb_ot_layout_script_get_language_tags(face, HB_OT_TAG_GPOS, i, 0, &langCount, langList);

                if (index < langCount) {
                    rval = langList[index];
                    break;
                }

                free(langList);
            }
        }
    }

    return rval;
}

hb_tag_t
getIndFeature(XeTeXFont font, hb_tag_t script, hb_tag_t language, unsigned int index)
{
    hb_tag_t rval = 0;

    hb_face_t* face = hb_font_get_face(((XeTeXFontInst*)font)->getHbFont());

    for (int i = 0; i < 2; ++i) {
        unsigned int scriptIndex, langIndex = 0;
        hb_tag_t tableTag = i == 0 ? HB_OT_TAG_GSUB : HB_OT_TAG_GPOS;
        if (hb_ot_layout_table_find_script(face, tableTag, script, &scriptIndex)) {
            if (hb_ot_layout_script_select_language(face, tableTag, scriptIndex, 1, &language, &langIndex) || language == 0) {
                unsigned int featCount = hb_ot_layout_language_get_feature_tags(face, tableTag, scriptIndex, langIndex, 0, NULL, NULL);
                hb_tag_t* featList = (hb_tag_t*) xcalloc(featCount, sizeof(hb_tag_t*));
                hb_ot_layout_language_get_feature_tags(face, tableTag, scriptIndex, langIndex, 0, &featCount, featList);

                if (index < featCount) {
                    rval = featList[index];
                    break;
                }

                index -= featCount;
            }
        }
    }

    return rval;
}

uint32_t
countGraphiteFeatures(XeTeXLayoutEngine engine)
{
    uint32_t rval = 0;

    hb_face_t* hbFace = hb_font_get_face(engine->font->getHbFont());
    gr_face* grFace = hb_graphite2_face_get_gr_face(hbFace);

    if (grFace != NULL)
        rval = gr_face_n_fref(grFace);

    return rval;
}

uint32_t
getGraphiteFeatureCode(XeTeXLayoutEngine engine, uint32_t index)
{
    uint32_t rval = 0;

    hb_face_t* hbFace = hb_font_get_face(engine->font->getHbFont());
    gr_face* grFace = hb_graphite2_face_get_gr_face(hbFace);

    if (grFace != NULL) {
        const gr_feature_ref* feature = gr_face_fref(grFace, index);
        rval = gr_fref_id(feature);
    }

    return rval;
}

uint32_t
countGraphiteFeatureSettings(XeTeXLayoutEngine engine, uint32_t featureID)
{
    uint32_t rval = 0;

    hb_face_t* hbFace = hb_font_get_face(engine->font->getHbFont());
    gr_face* grFace = hb_graphite2_face_get_gr_face(hbFace);

    if (grFace != NULL) {
        const gr_feature_ref* feature = gr_face_find_fref(grFace, featureID);
        rval = gr_fref_n_values(feature);
    }

    return rval;
}

uint32_t
getGraphiteFeatureSettingCode(XeTeXLayoutEngine engine, uint32_t featureID, uint32_t index)
{
    uint32_t rval = 0;

    hb_face_t* hbFace = hb_font_get_face(engine->font->getHbFont());
    gr_face* grFace = hb_graphite2_face_get_gr_face(hbFace);

    if (grFace != NULL) {
        const gr_feature_ref* feature = gr_face_find_fref(grFace, featureID);
        rval = gr_fref_value(feature, index);
    }

    return rval;
}

hb_tag_t tag_from_lang(hb_language_t lang) {
	const char* str = hb_language_to_string(lang);
	if (str) {
		return hb_tag_from_string(str, strlen(str));
	} else {
		return 0;
	}
}

uint32_t
getGraphiteFeatureDefaultSetting(XeTeXLayoutEngine engine, uint32_t featureID)
{
    uint32_t rval = 0;

    hb_face_t* hbFace = hb_font_get_face(engine->font->getHbFont());
    gr_face* grFace = hb_graphite2_face_get_gr_face(hbFace);

    if (grFace != NULL) {
        const gr_feature_ref* feature = gr_face_find_fref(grFace, featureID);
        gr_feature_val *featureValues = gr_face_featureval_for_lang (grFace, tag_from_lang(engine->language));

        rval = gr_fref_feature_value(feature, featureValues);
    }

    return rval;
}

char *
getGraphiteFeatureLabel(XeTeXLayoutEngine engine, uint32_t featureID)
{
    hb_face_t* hbFace = hb_font_get_face(engine->font->getHbFont());
    gr_face* grFace = hb_graphite2_face_get_gr_face(hbFace);

    if (grFace != NULL) {
        const gr_feature_ref* feature = gr_face_find_fref(grFace, featureID);
        uint32_t len = 0;
        uint16_t langID = 0x409;

        return (char *) gr_fref_label(feature, &langID, gr_utf8, &len);
    }

    return NULL;
}

char *
getGraphiteFeatureSettingLabel(XeTeXLayoutEngine engine, uint32_t featureID, uint32_t settingID)
{
    hb_face_t* hbFace = hb_font_get_face(engine->font->getHbFont());
    gr_face* grFace = hb_graphite2_face_get_gr_face(hbFace);

    if (grFace != NULL) {
        const gr_feature_ref* feature = gr_face_find_fref(grFace, featureID);
        for (int i = 0; i < gr_fref_n_values(feature); i++) {
            if ((int) settingID == gr_fref_value(feature, i)) {
                uint32_t len = 0;
                uint16_t langID = 0x409;

                return (char *) gr_fref_value_label(feature, i, &langID, gr_utf8, &len);
            }
        }
    }

    return NULL;
}

bool
findGraphiteFeature(XeTeXLayoutEngine engine, const char* s, const char* e, hb_tag_t* f, int* v)
    /* s...e is a "feature=setting" string; look for this in the font */
{
    long tmp;

    *f = 0;
    *v = 0;
    while (*s == ' ' || *s == '\t')
        ++s;
    const char* cp = s;
    while (cp < e && *cp != '=')
        ++cp;

    tmp = findGraphiteFeatureNamed(engine, s, cp - s);
    *f = tmp;
    if (tmp == -1)
        return false;

    ++cp;
    while (cp < e && (*cp == ' ' || *cp == '\t'))
        ++cp;

    if (cp == e)
        /* no setting was specified */
        return false;

    *v = findGraphiteFeatureSettingNamed(engine, *f, cp, e - cp);
    if (*v == -1)
        return false;

    return true;
}

long
findGraphiteFeatureNamed(XeTeXLayoutEngine engine, const char* name, int namelength)
{
    long rval = -1;

    hb_face_t* hbFace = hb_font_get_face(engine->font->getHbFont());
    gr_face* grFace = hb_graphite2_face_get_gr_face(hbFace);

    if (grFace != NULL) {
    	hb_tag_t tag = hb_tag_from_string(name, namelength);
        for (int i = 0; i < gr_face_n_fref(grFace); i++) {
            const gr_feature_ref* feature = gr_face_fref(grFace, i);
            uint32_t len = 0;
            uint16_t langID = 0x409;

            char* label = (char*)gr_fref_label(feature, &langID, gr_utf8, &len);

            if (gr_fref_id(feature) == tag || strncmp(label, name, namelength) == 0) {
                rval = gr_fref_id(feature);
                gr_label_destroy(label);
                break;
            }

            gr_label_destroy(label);
        }
    }

    return rval;
}

long
findGraphiteFeatureSettingNamed(XeTeXLayoutEngine engine, uint32_t id, const char* name, int namelength)
{
    long rval = -1;

    hb_face_t* hbFace = hb_font_get_face(engine->font->getHbFont());
    gr_face* grFace = hb_graphite2_face_get_gr_face(hbFace);

    if (grFace != NULL) {
    	hb_tag_t tag = hb_tag_from_string(name, namelength);
        const gr_feature_ref* feature = gr_face_find_fref(grFace, id);
        for (int i = 0; i < gr_fref_n_values(feature); i++) {
            uint32_t len = 0;
            uint16_t langID = 0x409;

            char* label = (char*)gr_fref_value_label(feature, i, &langID, gr_utf8, &len);

            if (gr_fref_id(feature) == tag || strncmp(label, name, namelength) == 0) {
                rval = gr_fref_value(feature, i);
                gr_label_destroy(label);
                break;
            }

            gr_label_destroy(label);
        }
    }

    return rval;
}

XeTeXFont
getFont(XeTeXLayoutEngine engine)
{
    return (XeTeXFont)(engine->font);
}

float
getExtendFactor(XeTeXLayoutEngine engine)
{
    return engine->extend;
}

float
getSlantFactor(XeTeXLayoutEngine engine)
{
    return engine->slant;
}

float
getEmboldenFactor(XeTeXLayoutEngine engine)
{
    return engine->embolden;
}

XeTeXLayoutEngine
createLayoutEngine(PlatformFontRef fontRef, XeTeXFont font, hb_tag_t script, char *language,
                    hb_feature_t* features, int nFeatures, char **shapers, uint32_t rgbValue,
                    float extend, float slant, float embolden)
{
    XeTeXLayoutEngine result = new XeTeXLayoutEngine_rec;
    result->fontRef = fontRef;
    result->font = (XeTeXFontInst*)font;
    result->script = script;
    result->features = features;
    result->ShaperList = shapers;
    result->shaperListToFree = false;
    result->shaper = NULL;
    result->nFeatures = nFeatures;
    result->rgbValue = rgbValue;
    result->extend = extend;
    result->slant = slant;
    result->embolden = embolden;
    result->hbBuffer = hb_buffer_create();

    // For Graphite fonts treat the language as BCP 47 tag, for OpenType we
    // treat it as a OT language tag for backward compatibility with pre-0.9999
    // XeTeX.
    if (getReqEngine() == 'G')
        result->language = hb_language_from_string(language, -1);
    else
        result->language = hb_ot_tag_to_language(hb_tag_from_string(language, -1));

    free(language);

    return result;
}

void
deleteLayoutEngine(XeTeXLayoutEngine engine)
{
    hb_buffer_destroy(engine->hbBuffer);
    deleteFont((XeTeXFont)engine->font);
    free(engine->shaper);
    if(engine->shaperListToFree) {
      free(engine->ShaperList);
      engine->shaperListToFree = false;
      engine->ShaperList = NULL;
    }
    delete engine;
}

#if !HB_VERSION_ATLEAST(2,5,0)
static unsigned int
_decompose_compat(hb_unicode_funcs_t* ufuncs,
                  hb_codepoint_t      u,
                  hb_codepoint_t*     decomposed,
                  void*               user_data)
{
    return 0;
}

static hb_unicode_funcs_t*
_get_unicode_funcs(void)
{
    static hb_unicode_funcs_t* ufuncs = hb_unicode_funcs_create(hb_icu_get_unicode_funcs());
    hb_unicode_funcs_set_decompose_compatibility_func(ufuncs, _decompose_compat, NULL, NULL);
    return ufuncs;
}
#endif

int
layoutChars(XeTeXLayoutEngine engine, uint16_t chars[], int32_t offset, int32_t count, int32_t max,
                        bool rightToLeft)
{
    bool res;
    hb_script_t script = HB_SCRIPT_INVALID;
    hb_direction_t direction = HB_DIRECTION_LTR;
    hb_segment_properties_t segment_props;
    hb_shape_plan_t *shape_plan;
    hb_font_t* hbFont = engine->font->getHbFont();
    hb_face_t* hbFace = hb_font_get_face(hbFont);

    if (engine->font->getLayoutDirVertical())
        direction = HB_DIRECTION_TTB;
    else if (rightToLeft)
        direction = HB_DIRECTION_RTL;

    script = hb_ot_tag_to_script (engine->script);

    hb_buffer_reset(engine->hbBuffer);

#if !HB_VERSION_ATLEAST(2,5,0)
    static hb_unicode_funcs_t* hbUnicodeFuncs = NULL;
    if (hbUnicodeFuncs == NULL)
        hbUnicodeFuncs = _get_unicode_funcs();
    hb_buffer_set_unicode_funcs(engine->hbBuffer, hbUnicodeFuncs);
#endif

    hb_buffer_add_utf16(engine->hbBuffer, chars, max, offset, count);
    hb_buffer_set_direction(engine->hbBuffer, direction);
    hb_buffer_set_script(engine->hbBuffer, script);
    hb_buffer_set_language(engine->hbBuffer, engine->language);

    hb_buffer_guess_segment_properties(engine->hbBuffer);
    hb_buffer_get_segment_properties(engine->hbBuffer, &segment_props);

    if (engine->ShaperList == NULL) {
        // HarfBuzz gives graphite2 shaper a priority, so that for hybrid
        // Graphite/OpenType fonts, Graphite will be used. However, pre-0.9999
        // XeTeX preferred OpenType over Graphite, so we are doing the same
        // here for sake of backward compatibility. Since "ot" shaper never
        // fails, we set the shaper list to just include it.
        engine->ShaperList = (char**) xcalloc(2, sizeof(char*));
        engine->ShaperList[0] = (char*) "ot";
        engine->ShaperList[1] = NULL;
        engine->shaperListToFree = true;
    }

    shape_plan = hb_shape_plan_create_cached(hbFace, &segment_props, engine->features, engine->nFeatures, engine->ShaperList);
    res = hb_shape_plan_execute(shape_plan, hbFont, engine->hbBuffer, engine->features, engine->nFeatures);

    if (engine->shaper != NULL) {
        free(engine->shaper);
        engine->shaper = NULL;
    }

    if (res) {
        engine->shaper = strdup(hb_shape_plan_get_shaper(shape_plan));
        hb_buffer_set_content_type(engine->hbBuffer, HB_BUFFER_CONTENT_TYPE_GLYPHS);
    } else {
        // all selected shapers failed, retrying with default
        // we don't use _cached here as the cached plain will always fail.
        hb_shape_plan_destroy(shape_plan);
        shape_plan = hb_shape_plan_create(hbFace, &segment_props, engine->features, engine->nFeatures, NULL);
        res = hb_shape_plan_execute(shape_plan, hbFont, engine->hbBuffer, engine->features, engine->nFeatures);

        if (res) {
            engine->shaper = strdup(hb_shape_plan_get_shaper(shape_plan));
            hb_buffer_set_content_type(engine->hbBuffer, HB_BUFFER_CONTENT_TYPE_GLYPHS);
        } else {
            _tt_abort("all shapers failed");
        }
    }

    hb_shape_plan_destroy(shape_plan);

    int glyphCount = hb_buffer_get_length(engine->hbBuffer);

#ifdef DEBUG
    char buf[1024];
    unsigned int consumed;

    printf ("shaper: %s\n", engine->shaper);

    hb_buffer_serialize_flags_t flags = HB_BUFFER_SERIALIZE_FLAGS_DEFAULT;
    hb_buffer_serialize_format_t format = HB_BUFFER_SERIALIZE_FORMAT_JSON;

    hb_buffer_serialize_glyphs (engine->hbBuffer, 0, glyphCount, buf, sizeof(buf), &consumed, hbFont, format, flags);
    if (consumed)
        printf ("buffer glyphs: %s\n", buf);
#endif

    return glyphCount;
}

void
getGlyphs(XeTeXLayoutEngine engine, uint32_t glyphs[])
{
    int glyphCount = hb_buffer_get_length(engine->hbBuffer);
    hb_glyph_info_t *hbGlyphs = hb_buffer_get_glyph_infos(engine->hbBuffer, NULL);

    for (int i = 0; i < glyphCount; i++)
        glyphs[i] = hbGlyphs[i].codepoint;
}

void
getGlyphAdvances(XeTeXLayoutEngine engine, float advances[])
{
    int glyphCount = hb_buffer_get_length(engine->hbBuffer);
    hb_glyph_position_t *hbPositions = hb_buffer_get_glyph_positions(engine->hbBuffer, NULL);

    for (int i = 0; i < glyphCount; i++) {
        if (engine->font->getLayoutDirVertical())
            advances[i] = engine->font->unitsToPoints(hbPositions[i].y_advance);
        else
            advances[i] = engine->font->unitsToPoints(hbPositions[i].x_advance);
    }
}

void
getGlyphPositions(XeTeXLayoutEngine engine, FloatPoint positions[])
{
    int glyphCount = hb_buffer_get_length(engine->hbBuffer);
    hb_glyph_position_t *hbPositions = hb_buffer_get_glyph_positions(engine->hbBuffer, NULL);

    float x = 0, y = 0;

    if (engine->font->getLayoutDirVertical()) {
        for (int i = 0; i < glyphCount; i++) {
            positions[i].x = -engine->font->unitsToPoints(x + hbPositions[i].y_offset); /* negative is forwards */
            positions[i].y =  engine->font->unitsToPoints(y - hbPositions[i].x_offset);
            x += hbPositions[i].y_advance;
            y += hbPositions[i].x_advance;
        }
        positions[glyphCount].x = -engine->font->unitsToPoints(x);
        positions[glyphCount].y =  engine->font->unitsToPoints(y);
    } else {
        for (int i = 0; i < glyphCount; i++) {
            positions[i].x =  engine->font->unitsToPoints(x + hbPositions[i].x_offset);
            positions[i].y = -engine->font->unitsToPoints(y + hbPositions[i].y_offset); /* negative is upwards */
            x += hbPositions[i].x_advance;
            y += hbPositions[i].y_advance;
        }
        positions[glyphCount].x =  engine->font->unitsToPoints(x);
        positions[glyphCount].y = -engine->font->unitsToPoints(y);
    }

    if (engine->extend != 1.0 || engine->slant != 0.0)
        for (int i = 0; i <= glyphCount; ++i)
            positions[i].x = positions[i].x * engine->extend - positions[i].y * engine->slant;
}

float
getPointSize(XeTeXLayoutEngine engine)
{
    return engine->font->getPointSize();
}

void
getAscentAndDescent(XeTeXLayoutEngine engine, float* ascent, float* descent)
{
    *ascent = engine->font->getAscent();
    *descent = engine->font->getDescent();
}

void
getCapAndXHeight(XeTeXLayoutEngine engine, float* capheight, float* xheight)
{
    *capheight = engine->font->getCapHeight();
    *xheight = engine->font->getXHeight();
}

int
getDefaultDirection(XeTeXLayoutEngine engine)
{
    hb_script_t script = hb_buffer_get_script(engine->hbBuffer);
    if (hb_script_get_horizontal_direction (script) == HB_DIRECTION_RTL)
        return UBIDI_DEFAULT_RTL;
    else
        return UBIDI_DEFAULT_LTR;
}

uint32_t
getRgbValue(XeTeXLayoutEngine engine)
{
    return engine->rgbValue;
}

void
getGlyphBounds(XeTeXLayoutEngine engine, uint32_t glyphID, GlyphBBox* bbox)
{
    engine->font->getGlyphBounds(glyphID, bbox);
    if (engine->extend != 0.0) {
        bbox->xMin *= engine->extend;
        bbox->xMax *= engine->extend;
    }
}

float
getGlyphWidthFromEngine(XeTeXLayoutEngine engine, uint32_t glyphID)
{
    return engine->extend * getGlyphWidth((XeTeXFont)engine->font, glyphID);
}

void
getGlyphHeightDepth(XeTeXLayoutEngine engine, uint32_t glyphID, float* height, float* depth)
{
    engine->font->getGlyphHeightDepth(glyphID, height, depth);
}

void
getGlyphSidebearings(XeTeXLayoutEngine engine, uint32_t glyphID, float* lsb, float* rsb)
{
    engine->font->getGlyphSidebearings(glyphID, lsb, rsb);
    if (engine->extend != 0.0) {
        *lsb *= engine->extend;
        *rsb *= engine->extend;
    }
}

float
getGlyphItalCorr(XeTeXLayoutEngine engine, uint32_t glyphID)
{
    return engine->extend * engine->font->getGlyphItalCorr(glyphID);
}

uint32_t
mapCharToGlyph(XeTeXLayoutEngine engine, uint32_t charCode)
{
    return engine->font->mapCharToGlyph(charCode);
}

int
getFontCharRange(XeTeXLayoutEngine engine, int reqFirst)
{
    if (reqFirst)
        return engine->font->getFirstCharCode();
    else
        return engine->font->getLastCharCode();
}

const char*
getGlyphName(XeTeXFont font, uint16_t gid, int* len)
{
    return ((XeTeXFontInst*)font)->getGlyphName(gid, *len);
}

int
mapGlyphToIndex(XeTeXLayoutEngine engine, const char* glyphName)
{
    return engine->font->mapGlyphToIndex(glyphName);
}

static gr_segment* grSegment = NULL;
static const gr_slot* grPrevSlot = NULL;
static int grTextLen;

bool
initGraphiteBreaking(XeTeXLayoutEngine engine, const uint16_t* txtPtr, int txtLen)
{
    hb_font_t* hbFont = engine->font->getHbFont();
    hb_face_t* hbFace = hb_font_get_face(hbFont);
    gr_face* grFace = hb_graphite2_face_get_gr_face(hbFace);
    gr_font* grFont = gr_make_font(hb_font_get_ptem(hbFont), grFace);
    if (grFace != NULL && grFont != NULL) {
        if (grSegment != NULL) {
            gr_seg_destroy(grSegment);
            grSegment = NULL;
            grPrevSlot = NULL;
        }

        gr_feature_val *grFeatureValues = gr_face_featureval_for_lang (grFace, tag_from_lang(engine->language));

        int nFeatures = engine->nFeatures;
        hb_feature_t *features =  engine->features;
        while (nFeatures--) {
            const gr_feature_ref *fref = gr_face_find_fref (grFace, features->tag);
            if (fref)
                gr_fref_set_feature_value (fref, features->value, grFeatureValues);
            features++;
        }

        grSegment = gr_make_seg(grFont, grFace, engine->script, grFeatureValues, gr_utf16, txtPtr, txtLen, 0);
        grPrevSlot = gr_seg_first_slot(grSegment);
        grTextLen = txtLen;

        return true;
    }

    return false;
}

int
findNextGraphiteBreak(void)
{
    int ret = -1;

    if (grSegment != NULL) {
        if (grPrevSlot && grPrevSlot != gr_seg_last_slot(grSegment)) {
            for (const gr_slot* s = gr_slot_next_in_segment(grPrevSlot); s != NULL; s = gr_slot_next_in_segment(s)) {
                const gr_char_info* ci = NULL;
                int bw;

                ci = gr_seg_cinfo(grSegment, gr_slot_index(s));
                bw = gr_cinfo_break_weight(ci);
                if (bw < gr_breakNone && bw >= gr_breakBeforeWord) {
                    grPrevSlot = s;
                    ret = gr_cinfo_base(ci);
                } else if (bw > gr_breakNone && bw <= gr_breakWord) {
                    grPrevSlot = gr_slot_next_in_segment(s);
                    ret = gr_cinfo_base(ci) + 1;
                }

                if (ret != -1)
                    break;
            }

            if (ret == -1) {
                grPrevSlot = gr_seg_last_slot(grSegment);
                ret = grTextLen;
            }
        }
    }

    return ret;
}

bool
usingGraphite(XeTeXLayoutEngine engine)
{
    if (engine->shaper != NULL && (strcmp("graphite2", engine->shaper) == 0))
        return true;
    else
        return false;
}

bool
usingOpenType(XeTeXLayoutEngine engine)
{
    if (engine->shaper == NULL || (strcmp("ot", engine->shaper) == 0))
        return true;
    else
        return false;
}

bool
isOpenTypeMathFont(XeTeXLayoutEngine engine)
{
    return hb_ot_math_has_data(hb_font_get_face(engine->font->getHbFont()));
}

/* New Tectonic APIs for crate encapsulation */

hb_font_t *
ttxl_get_hb_font(XeTeXLayoutEngine engine)
{
    return engine->font->getHbFont();
}

float
ttxl_font_units_to_points(XeTeXFont font, float units)
{
    return ((XeTeXFontInst *) font)->unitsToPoints(units);
}

float
ttxl_font_points_to_units(XeTeXFont font, float points)
{
    return ((XeTeXFontInst *) font)->pointsToUnits(points);
}

float
ttxl_font_get_point_size(XeTeXFont font)
{
    return ((XeTeXFontInst *) font)->getPointSize();
}

const char *
ttxl_platfont_get_desc(PlatformFontRef fontRef)
{
    return XeTeXFontMgr::GetFontManager()->getPlatformFontDesc(fontRef).c_str();
}
