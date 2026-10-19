#[cfg(debug_assertions)]
use crate::analyzer::types::{PrimitiveType, StaticType};
use crate::types::*;

pub trait AnalogType {
    const STATIC_TY: StaticType<'static>;
}
macro_rules! primitive_types {
    ($($Type:ty = $StaticType:ident;)*) => {$(
        impl AnalogType for $Type {
            const STATIC_TY: StaticType<'static> = StaticType::Primitive(PrimitiveType::$StaticType);
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

#[derive(Clone, PartialEq, PartialOrd, Default)]
pub struct Value<'val, 'src> {
    pub data: &'val [u8],

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
                                PrimitiveType::None => self.0.clone().cast_into::<none>(),
                                PrimitiveType::Nevr => self.0.clone().cast_into::<nevr>(),
                                PrimitiveType::Fail => self.0.clone().cast_into::<fail>(),
                                PrimitiveType::Bool => self.0.clone().cast_into::<bool>(),
                                PrimitiveType::Byte => self.0.clone().cast_into::<byte>(),
                                PrimitiveType::Uint => self.0.clone().cast_into::<uint>(),
                                PrimitiveType::Sint => self.0.clone().cast_into::<sint>(),
                                PrimitiveType::Flpt => self.0.clone().cast_into::<flpt>(),
                                PrimitiveType::Char => self.0.clone().cast_into::<char>(),
                                PrimitiveType::Text => self.0.clone().cast_into::<text>(),
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
                        | StaticType::Fun(_) => &"[unknown]",
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

impl<'val, 'src> Value<'val, 'src> {
    pub const fn new(data: &'val [u8], #[cfg(debug_assertions)] tag_: StaticType<'src>) -> Self {
        Self {
            data,
            #[cfg(debug_assertions)]
            tag_,
            #[cfg(not(debug_assertions))]
            _marker: PhantomData,
        }
    }

    pub const fn cast_from<T: AnalogType>(value: &'val T) -> Self {
        let len = std::mem::size_of_val(value);
        let data = (&raw const *value).cast();
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
    pub unsafe fn cast_into<T: AnalogType>(self) -> &'val T {
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
        let ptr = self.data.as_ptr().cast::<T>();
        debug_assert!(ptr.is_aligned(), "pointer should be aligned");
        // SAFETY: caller must uphold safety contract
        unsafe { &*ptr }
    }
}

#[derive(PartialEq, PartialOrd, Default)]
pub struct ValueMut<'val, 'src> {
    pub data: &'val mut [u8],

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

impl<'val, 'src> ValueMut<'val, 'src> {
    pub const fn new(
        data: &'val mut [u8],
        #[cfg(debug_assertions)] tag_: StaticType<'src>,
    ) -> Self {
        Self {
            data,
            #[cfg(debug_assertions)]
            tag_,
            #[cfg(not(debug_assertions))]
            _marker: PhantomData,
        }
    }

    pub fn as_ref<'subval>(&'subval self) -> Value<'subval, 'src> {
        Value::new(
            &*self.data,
            #[cfg(debug_assertions)]
            self.tag_.clone(),
        )
    }

    pub const fn cast_from<T: AnalogType>(value: &'val mut T) -> Self {
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
    pub unsafe fn cast_into<T: AnalogType>(self) -> &'val mut T {
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
        let ptr = self.data.as_mut_ptr().cast::<T>();
        debug_assert!(ptr.is_aligned(), "pointer should be aligned");
        // SAFETY: caller must uphold safety contract
        unsafe { &mut *ptr }
    }
}
