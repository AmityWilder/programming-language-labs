use super::debug_unsafe;
#[cfg(debug_assertions)]
use crate::analyzer::types::StaticType;
use crate::types::{byte, fail, flpt, nevr, none, sint, text, uint};
use std::mem::MaybeUninit;

pub trait AnalogType {
    const STATIC_TY: StaticType<'static>;
}
macro_rules! primitive_types {
    ($($Type:ty = $StaticType:ident;)*) => {$(
        impl AnalogType for $Type {
            const STATIC_TY: StaticType<'static> = StaticType::$StaticType;
        }
    )*};
}
primitive_types! {
    none = None;
    nevr = Nevr;
    fail = Fail;
    byte = Byte;
    bool = Bool;
    uint = Uint;
    sint = Sint;
    flpt = Flpt;
    char = Char;
    text = Text;
}

#[derive(Clone)]
pub struct Value<'val, 'src, T: ?Sized = [MaybeUninit<u8>]> {
    pub data: &'val T,

    /// For ensuring type checker is sound
    #[cfg(debug_assertions)]
    pub tag_: StaticType<'src>,

    #[cfg(not(debug_assertions))]
    _marker: PhantomData<&'src ()>,
}

impl std::fmt::Debug for Value<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(debug_assertions)]
        struct SpecValue<'a, 'b, 'c>(&'a Value<'b, 'c>);

        #[cfg(debug_assertions)]
        impl std::fmt::Debug for SpecValue<'_, '_, '_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                // SAFETY: static analysis should ensure this
                unsafe {
                    let value: &dyn std::fmt::Debug = match &self.0.tag_ {
                        StaticType::Primitive(t) => {
                            use crate::analyzer::types::PrimitiveType;
                            match t {
                                PrimitiveType::None => &*self.0.data.as_ptr().cast::<none>(),
                                PrimitiveType::Nevr => &*self.0.data.as_ptr().cast::<nevr>(),
                                PrimitiveType::Fail => &*self.0.data.as_ptr().cast::<fail>(),
                                PrimitiveType::Bool => &*self.0.data.as_ptr().cast::<bool>(),
                                PrimitiveType::Byte => &*self.0.data.as_ptr().cast::<byte>(),
                                PrimitiveType::Uint => &*self.0.data.as_ptr().cast::<uint>(),
                                PrimitiveType::Sint => &*self.0.data.as_ptr().cast::<sint>(),
                                PrimitiveType::Flpt => &*self.0.data.as_ptr().cast::<flpt>(),
                                PrimitiveType::Char => &*self.0.data.as_ptr().cast::<char>(),
                                PrimitiveType::Text => &*self.0.data.as_ptr().cast::<text>(),
                            }
                        }
                        StaticType::View(_)
                        | StaticType::Arr(_)
                        | StaticType::Ptr(_)
                        | StaticType::OrTy(_)
                        | StaticType::Rec(_)
                        | StaticType::Sup(_)
                        | StaticType::Sub(_)
                        | StaticType::Cat(_)
                        | StaticType::Fun(_) => &"[unknown]", // TODO
                    };
                    value.fmt(f)
                }
            }
        }

        let mut f = f.debug_struct("Value");
        #[cfg(debug_assertions)]
        let f = f.field("tag_", &self.tag_).field("data", &SpecValue(self));
        f.field("data(raw)", &self.data).finish()
    }
}

impl<'val, 'src, T: ?Sized> Value<'val, 'src, T> {
    pub const fn new(data: &'val T, #[cfg(debug_assertions)] tag_: StaticType<'src>) -> Self {
        Self {
            data,
            #[cfg(debug_assertions)]
            tag_,
            #[cfg(not(debug_assertions))]
            _marker: PhantomData,
        }
    }
}

impl<'val, 'src, T: AnalogType> Value<'val, 'src, MaybeUninit<T>> {
    /// # Safety
    /// See [`MaybeUninit::assume_init_read`]
    pub unsafe fn assume_init(self) -> T {
        debug_unsafe!("assuming initialized: {:?}", self.data);
        // SAFETY: caller must uphold safety contract
        unsafe { self.data.assume_init_read() }
    }
}

impl<'val, 'src> Value<'val, 'src> {
    pub const fn cast_from<T: ?Sized + AnalogType>(value: &'val T) -> Self {
        let len = std::mem::size_of_val(value);
        let data = (&raw const *value).cast::<MaybeUninit<u8>>();
        // SAFETY: `data` comes from a reference to valid memory of len `len`
        let data = unsafe { std::slice::from_raw_parts(data, len) };
        Self::new(
            data,
            #[cfg(debug_assertions)]
            T::STATIC_TY,
        )
    }

    /// # Safety
    /// `self` must hold a pointer to a legitimate bytestring representation of
    /// a valid instance of `T`, or a type that is safe to cast to `T`.
    pub unsafe fn cast_into<T: AnalogType>(self) -> Value<'val, 'src, MaybeUninit<T>> {
        #[cfg(debug_assertions)]
        debug_assert_eq!(
            T::STATIC_TY,
            self.tag_,
            "SOUNDNESS ERROR: attempting to extract a different type (left) from what was inserted (right)"
        );
        // length in bytes, not elements
        let len = self.data.len();
        debug_assert_eq!(
            len,
            std::mem::size_of::<T>(),
            "type sizes must match in order to cast"
        );
        let ptr = self.data.as_ptr().cast::<MaybeUninit<T>>();
        debug_assert!(
            ptr.is_aligned(),
            "pointer should be aligned\n expected: {}\n offset: {}",
            std::mem::align_of::<T>(),
            ptr.align_offset(std::mem::align_of::<T>())
        );
        Value::new(
            // SAFETY: caller must uphold safety contract
            unsafe { &*ptr },
            #[cfg(debug_assertions)]
            self.tag_,
        )
    }
}

pub struct ValueMut<'val, 'src, T: ?Sized = [MaybeUninit<u8>]> {
    pub data: &'val mut T,

    /// For ensuring type checker is sound
    #[cfg(debug_assertions)]
    tag_: StaticType<'src>,

    #[cfg(not(debug_assertions))]
    _marker: PhantomData<&'src ()>,
}

impl std::fmt::Debug for ValueMut<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_ref().fmt(f)
    }
}

impl<'val, 'src, T: ?Sized> ValueMut<'val, 'src, T> {
    pub const fn new(data: &'val mut T, #[cfg(debug_assertions)] tag_: StaticType<'src>) -> Self {
        Self {
            data,
            #[cfg(debug_assertions)]
            tag_,
            #[cfg(not(debug_assertions))]
            _marker: PhantomData,
        }
    }

    pub fn as_ref<'subval>(&'subval self) -> Value<'subval, 'src, T> {
        Value::new(
            self.data,
            #[cfg(debug_assertions)]
            self.tag_.clone(),
        )
    }
}

impl<'val, 'src, T: AnalogType> ValueMut<'val, 'src, MaybeUninit<T>> {
    /// # Safety
    /// See [`MaybeUninit::assume_init_mut`]
    pub unsafe fn assume_init(self) -> ValueMut<'val, 'src, T> {
        debug_unsafe!("assuming initialized: {:?}", self.data);
        ValueMut::new(
            // SAFETY: caller must uphold safety contract
            unsafe { self.data.assume_init_mut() },
            #[cfg(debug_assertions)]
            self.tag_,
        )
    }
}

impl<'val, 'src> ValueMut<'val, 'src> {
    pub const fn cast_from<T: ?Sized + AnalogType>(value: &'val mut T) -> Self {
        let len = std::mem::size_of_val(value);
        let data = (&raw mut *value).cast();
        // SAFETY: `data` comes from a reference to valid memory of len `len`
        let data = unsafe { std::slice::from_raw_parts_mut(data, len) };
        Self::new(
            data,
            #[cfg(debug_assertions)]
            T::STATIC_TY,
        )
    }

    /// # Safety
    /// `self` must hold a pointer to a legitimate bytestring representation of
    /// a valid instance of `T`, or a type that is safe to cast to `T`.
    pub unsafe fn cast_into<T: AnalogType>(self) -> ValueMut<'val, 'src, MaybeUninit<T>> {
        #[cfg(debug_assertions)]
        debug_assert_eq!(
            T::STATIC_TY,
            self.tag_,
            "SOUNDNESS ERROR: attempting to extract a different type (left) from what was inserted (right)"
        );
        // length in bytes, not elements
        let len = self.data.len();
        debug_assert_eq!(
            len,
            std::mem::size_of::<T>(),
            "type sizes must match in order to cast"
        );
        let ptr = self.data.as_mut_ptr().cast::<MaybeUninit<T>>();
        debug_assert!(
            ptr.is_aligned(),
            "pointer should be aligned\n expected: {}\n offset: {}",
            std::mem::align_of::<T>(),
            ptr.align_offset(std::mem::align_of::<T>())
        );
        ValueMut::new(
            // SAFETY: caller must uphold safety contract
            unsafe { &mut *ptr },
            #[cfg(debug_assertions)]
            self.tag_,
        )
    }
}
