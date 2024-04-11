#[repr(C)]
pub struct CFAllocator(());

#[repr(C)]
pub struct CFDictionaryKeyCallBacks(());

#[repr(C)]
pub struct CFDictionaryValueCallBacks(());

#[repr(C)]
pub struct CFSetCallBacks(());

#[repr(C)]
pub struct CFArrayCallBacks(());

#[repr(C)]
pub struct CFDictionary(());

#[repr(C)]
pub struct CFSet(());

#[repr(C)]
pub struct CFArray(());

#[repr(C)]
pub struct CTFontDescriptor(());

#[repr(C)]
pub struct CTFont(());

#[repr(C)]
pub struct CFString(());

#[repr(C)]
pub struct CFURL(());

#[repr(C)]
pub struct CFRange {
    pub location: CFIndex,
    pub length: CFIndex,
}

#[repr(C)]
pub struct CGAffineTransform {
    a: CGFloat,
    b: CGFloat,
    c: CGFloat,
    d: CGFloat,
    tx: CGFloat,
    ty: CGFloat,
}

#[repr(C)]
pub struct NSFontManager(());

pub type CTFontDescriptorRef = *const CTFontDescriptor;
pub type CFDictionaryRef = *const CFDictionary;
pub type CFIndex = isize;
pub type CFTypeRef = *const ();
pub type CFAllocatorRef = *const CFAllocator;
pub type CFSetRef = *const CFSet;
pub type CFArrayRef = *const CFArray;
pub type CTFontRef = *const CTFont;
pub type CFStringRef = *const CFString;
pub type CFURLRef = *const CFURL;
pub type CFStringEncoding = u32;
#[cfg(target_os = "watchos")]
pub type CGFloat = f32;
#[cfg(not(target_os = "watchos"))]
pub type CGFloat = f64;

pub type CFTypeID = libc::c_ulong;

pub const kCFStringEncodingMacRoman: CFStringEncoding = 0;
pub const kCFStringEncodingASCII: CFStringEncoding = 0x0600;
pub const kCFStringEncodingNonLossyASCII: CFStringEncoding = 0x0BFF;
pub const kCFStringEncodingUnicode: CFStringEncoding = 0x0100;
pub const kCFStringEncodingUTF8: CFStringEncoding = 0x08000100;

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    pub fn CFSetCreate(
        allocator: CFAllocatorRef,
        values: *mut *const (),
        num_values: CFIndex,
        callbacks: *const CFSetCallBacks,
    ) -> CFSetRef;
    pub fn CFDictionaryCreate(
        allocator: CFAllocatorRef,
        keys: *mut *const (),
        values: *mut *const (),
        num_values: CFIndex,
        key_call_backs: *const CFDictionaryKeyCallBacks,
        value_call_backs: *const CFDictionaryValueCallBacks,
    ) -> CFDictionaryRef;
    pub fn CFRelease(cf: CFTypeRef);
    pub fn CFArrayGetCount(array: CFArrayRef) -> CFIndex;
    pub fn CFArrayGetValueAtIndex(array: CFArrayRef, idx: CFIndex) -> *const ();
    pub fn CFRetain(cf: CFTypeRef) -> CFTypeRef;
    pub fn CFArrayCreate(
        allocator: CFAllocatorRef,
        values: *mut *const (),
        num_values: CFIndex,
        call_backs: *const CFArrayCallBacks,
    ) -> CFArrayRef;
    pub fn CFStringGetLength(str: CFStringRef) -> CFIndex;
    pub fn CFStringGetCStringPtr(str: CFStringRef, enc: CFStringEncoding) -> *const libc::c_char;
    pub fn CFStringCreateWithCString(
        alloc: CFAllocatorRef,
        c_str: *const libc::c_char,
        encoding: CFStringEncoding,
    ) -> CFStringRef;
    pub fn CFStringCreateWithBytes(
        alloc: CFAllocatorRef,
        bytes: *const u8,
        num_bytes: CFIndex,
        encoding: CFStringEncoding,
        is_external: bool,
    ) -> CFStringRef;
    pub fn CFStringGetBytes(
        str: CFStringRef,
        range: CFRange,
        encoding: CFStringEncoding,
        loss_byte: u8,
        is_external: bool,
        buffer: *mut u8,
        max_buf_len: CFIndex,
        used_buf_len: *mut CFIndex,
    ) -> CFIndex;
    pub fn CFStringGetCString(
        str: CFStringRef,
        buffer: *mut libc::c_char,
        buffer_size: CFIndex,
        encoding: CFStringEncoding,
    ) -> bool;
    pub fn CFURLGetFileSystemRepresentation(
        url: CFURLRef,
        resolve_against_base: bool,
        buffer: *mut u8,
        max_buf_len: CFIndex,
    ) -> bool;
    pub fn CFSetGetTypeID() -> CFTypeID;
    pub fn CFArrayGetTypeID() -> CFTypeID;
    pub fn CFStringGetTypeID() -> CFTypeID;
    pub fn CFDictionaryGetTypeID() -> CFTypeID;
    pub fn CFURLGetTypeID() -> CFTypeID;
    pub fn CFGetTypeID(cf: CFTypeRef) -> CFTypeID;

    pub static kCFTypeDictionaryKeyCallBacks: CFDictionaryKeyCallBacks;
    pub static kCFTypeDictionaryValueCallBacks: CFDictionaryValueCallBacks;
    pub static kCFTypeSetCallBacks: CFSetCallBacks;
    pub static kCFTypeArrayCallBacks: CFArrayCallBacks;
}

#[link(name = "CoreText", kind = "framework")]
extern "C" {
    pub fn CTFontDescriptorCreateWithAttributes(attributes: CFDictionaryRef)
        -> CTFontDescriptorRef;
    pub fn CTFontDescriptorCreateMatchingFontDescriptors(
        descriptor: CTFontDescriptorRef,
        mandatory_attributes: CFSetRef,
    ) -> CFArrayRef;
    pub fn CTFontDescriptorCopyAttribute(
        descriptor: CTFontDescriptorRef,
        attribute: CFStringRef,
    ) -> CFTypeRef;
    pub fn CTFontCreateWithFontDescriptor(
        descriptor: CTFontDescriptorRef,
        size: CGFloat,
        matrix: *const CGAffineTransform,
    ) -> CTFontRef;
    pub fn CTFontCopyName(font: CTFontRef, name_key: CFStringRef) -> CFStringRef;
    pub fn CTFontCopyAttribute(font: CTFontRef, attribute: CFStringRef) -> CFTypeRef;
    pub fn CTFontDescriptorCreateCopyWithAttributes(
        original: CTFontDescriptorRef,
        attributes: CFDictionaryRef,
    ) -> CTFontDescriptorRef;
    pub fn CTFontCopyLocalizedName(
        font: CTFontRef,
        name_key: CFStringRef,
        actual_lang: *mut CFStringRef,
    ) -> CFStringRef;
    pub fn CTFontManagerCopyAvailableFontFamilyNames() -> CFArrayRef;
    pub fn CTFontGetTypeID() -> CFTypeID;
    pub fn CTFontDescriptorGetTypeID() -> CFTypeID;

    pub static kCTFontNameAttribute: CFStringRef;
    pub static kCTFontFullNameKey: CFStringRef;
    pub static kCTFontFamilyNameKey: CFStringRef;
    pub static kCTFontStyleNameKey: CFStringRef;
    pub static kCTFontURLAttribute: CFStringRef;
    pub static kCTFontPostScriptNameKey: CFStringRef;
    pub static kCTFontCascadeListAttribute: CFStringRef;
    pub static kCTFontFamilyNameAttribute: CFStringRef;
    pub static kCTFontDisplayNameAttribute: CFStringRef;
}
