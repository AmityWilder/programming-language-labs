#![allow(non_camel_case_types)]

#[expect(non_camel_case_types, reason = "consistency with usize/isize")]
pub type fsize = cfg_select! {
    target_pointer_width = "16" => f16,
    target_pointer_width = "32" => f32,
    target_pointer_width = "64" => f64,
};
const _: () = {
    assert!(
        std::mem::size_of::<fsize>() == std::mem::size_of::<usize>(),
        "fsize should be pointer-width"
    );
};

pub type none = ();
pub type nevr = !;
pub type fail = crate::exec::error::Failure;
// bool is already builtin
pub type byte = u8;
pub type uint = usize;
pub type sint = isize;
pub type flpt = fsize;
// char is already builtin
pub type text = String;
